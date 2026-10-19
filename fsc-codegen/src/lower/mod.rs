mod context;
mod declarations;

use crate::CodegenResult;
use context::LoweringContext;

pub(super) fn lower(script: &fsc_mir::Script) -> CodegenResult<fsb_ir::Program> {
    LoweringContext::new(script)?.lower_script()
}
