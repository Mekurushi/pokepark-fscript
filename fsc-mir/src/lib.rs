mod build;
mod ir;

pub use build::lower;
pub use fsc_sema::types::{AggregateField, FieldIdx, Ty, aggregate_fields};
pub use ir::{
    BasicBlock, BasicBlockId, BinaryOp, Body, ConstValue, Constant, DebugValue, FieldOperand,
    Function, FunctionId, FunctionKind, FunctionSignature, Linkage, LocalDecl, LocalId, Operand,
    Place, PlaceBase, ProjectionElem, RETURN_PLACE, Rvalue, START_BLOCK, Script, SourceInfo,
    Statement, StatementKind, StaticDecl, StaticId, Terminator, TerminatorKind, UnaryOp,
    VariableDebugInfo,
};

//TODO: optimization pass
//TODO: verification util
