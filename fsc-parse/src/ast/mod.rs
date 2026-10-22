use fsc_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    Vec3, //TODO: aggregate assignment
    Array { element: Box<Ty>, length: usize },
    Reference(Box<Ty>),
    Void,
    Bool,
    Str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    // arithmetic
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Not,       // !
    Neg,       // -
    AddressOf, // &
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateFieldInit {
    pub name: String,
    pub name_span: Span,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // TODO: resolve the signed, unsigned here cleanly; going with signed as default, until a
    // need for unsigned values comes up; since negation are separate tokens, this can't resolve
    // the whole signed ranges; fix
    IntLit(i32),
    FloatLit(f32),
    BoolLit(bool),
    StringLit(String),

    Var(String),

    Array {
        elements: Vec<Expr>,
    },

    Aggregate {
        ty: Ty,
        fields: Vec<AggregateFieldInit>,
    },

    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },

    Member {
        base: Box<Expr>,
        member: String,
        member_span: Span,
    },

    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: String,
        callee_span: Span,
        args: Vec<Expr>,
    },
    SysCall {
        args: Vec<Expr>,
    },
}

impl Expr {
    pub fn new(id: NodeId, kind: ExprKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub id: NodeId,
    pub kind: StmtKind,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum StmtKind {
    Return(Option<Expr>),
    Break,
    VarDecl {
        name: String,
        name_span: Span,
        ty: Ty,
        ty_span: Span,
        init: Option<Expr>,
    },
    ConstDecl(ConstDecl),
    Assign {
        target: Expr,
        expr: Expr,
    },
    ExprStmt(Expr),
    Pause(Expr),
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
}

impl Stmt {
    pub fn new(id: NodeId, kind: StmtKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub ty_span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionHeader {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
    pub ret_ty_span: Span,
}

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub id: NodeId,
    pub header: FunctionHeader,
    pub body: Vec<Stmt>,
    pub exported: bool,
}

#[derive(Debug, Clone)]
pub struct ExternFuncDecl {
    pub id: NodeId,
    pub header: FunctionHeader,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub ty_span: Span,
    pub initializer: Expr,
}

#[derive(Debug, Clone)]
pub struct ConfigDecl {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub ty_span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    FuncDef(FuncDef),
    ExternFuncDecl(ExternFuncDecl),
    ConstDecl(ConstDecl),
    ConfigDecl(ConfigDecl),
}

#[derive(Debug, Clone)]
pub struct Script {
    pub items: Vec<Item>,
}
