mod build;
mod ir;

pub use build::lower;
pub use ir::{
    BasicBlock, BasicBlockId, BinaryOp, Body, ConstValue, Constant, DebugValue, FieldOperand,
    Function, FunctionId, FunctionKind, FunctionSignature, Linkage, LocalDecl, LocalId, Operand,
    Place, ProjectionElem, RETURN_PLACE, Rvalue, START_BLOCK, Script, SourceInfo, Statement,
    StatementKind, Terminator, TerminatorKind, UnaryOp, VariableDebugInfo,
};

//TODO: optimization pass
//TODO: verification util
