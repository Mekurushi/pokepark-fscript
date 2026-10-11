mod local;
mod lower;
mod place;

pub use local::{Local, LocalId, LocalKind};
pub(crate) use lower::lower_script;
pub use place::{Place, ProjectionElem, Vec3Field};

use crate::types::Ty;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    //arithmetic
    Add,
    Sub,
    Mul,
    Div,
    // comparison
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    // logical
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit {
        value: i32,
        ty: Ty,
    },

    FloatLit {
        value: f32,
        ty: Ty,
    },

    BoolLit {
        value: bool,
        ty: Ty,
    },
    StrLit {
        value: String,
        ty: Ty,
    },

    Load {
        place: Place,
        ty: Ty,
    },

    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        ty: Ty,
    },

    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        ty: Ty,
    },

    Call {
        callee: String,
        args: Vec<Expr>,
        ty: Ty,
    },
    SysCall {
        page: u8,
        func: u16,
        subtype: u8,
        args: Vec<SysCallArg>,
        ty: Ty,
    },
}

#[derive(Debug, Clone)]
pub enum SysCallArg {
    Value(Expr),
    StackRef(Place),
}

impl Expr {
    pub fn ty(&self) -> &Ty {
        match self {
            Self::IntLit { ty, .. }
            | Self::FloatLit { ty, .. }
            | Self::Load { ty, .. }
            | Self::BinOp { ty, .. }
            | Self::BoolLit { ty, .. }
            | Self::Unary { ty, .. }
            | Self::Call { ty, .. }
            | Self::StrLit { ty, .. }
            | Self::SysCall { ty, .. } => ty,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Return(Expr),
    Break,

    ReturnVoid,
    VarDecl {
        local: LocalId,
        init: Option<Expr>,
    },
    Assign {
        target: Place,
        value: Expr,
    },
    ExprStmt(Expr),
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Pause(Expr),
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub exported: bool,
    pub ret_ty: Ty,
    pub locals: Vec<Local>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Item {
    FuncDef(FuncDef),
}

#[derive(Debug)]
pub struct Script {
    pub items: Vec<Item>,
}
