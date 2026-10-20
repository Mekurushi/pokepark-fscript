mod block;
mod block_mapping;
mod call;
mod context;
mod declarations;
mod function;
mod function_mapping;
mod operand;
mod raw_syscall;
mod rvalue;
mod statement;
mod terminator;

use crate::CodegenResult;
use context::ScriptLowerer;

pub(super) fn lower(script: &fsc_mir::Script) -> CodegenResult<fsb_ir::Program> {
    ScriptLowerer::new(script)?.lower_script()
}
