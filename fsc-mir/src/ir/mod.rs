mod body;
mod control_flow;
mod debug;
mod entity;
mod function;
mod place;
mod value;

pub use body::{Body, LocalDecl};
pub use control_flow::{BasicBlock, Statement, StatementKind, Terminator, TerminatorKind};
pub use debug::{DebugValue, VariableDebugInfo};
pub use entity::{BasicBlockId, FunctionId, LocalId, RETURN_PLACE, START_BLOCK};
pub use function::{Function, FunctionKind, FunctionSignature, Linkage, Script};
pub use place::{Place, ProjectionElem};
pub use value::{BinaryOp, ConstValue, Constant, FieldOperand, Operand, Rvalue, UnaryOp};

use fsc_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceInfo {
    pub span: Span,
}
