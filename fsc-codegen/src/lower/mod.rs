mod block;
mod block_mapping;
mod body;
mod context;
mod declarations;
mod operand;
mod rvalue;
mod statement;
mod terminator;

use crate::CodegenResult;
use context::LoweringContext;

pub(super) fn lower(script: &fsc_mir::Script) -> CodegenResult<fsb_ir::Program> {
    LoweringContext::new(script)?.lower_script()
}
