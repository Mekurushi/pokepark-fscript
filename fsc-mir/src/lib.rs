mod build;
mod ir;

pub use build::lower;
pub use fsc_sema::types::{aggregate_fields, AggregateField, FieldIdx, Ty};
pub use ir::{
    BasicBlock, BasicBlockId, BinaryOp, Body, CallArgument, ConstValue, Constant, DebugValue,
    FieldOperand, Function, FunctionId, FunctionKind, FunctionSignature, Linkage, LocalDecl,
    LocalId, LocalKind, Operand, Place, PlaceBase, ProjectionElem, Rvalue, Script, SourceInfo,
    Statement, StatementKind, StaticDecl, StaticId, Terminator, TerminatorKind, UnaryOp,
    VariableDebugInfo, RETURN_PLACE, START_BLOCK,
};

//TODO: optimization pass
//TODO: verification util
