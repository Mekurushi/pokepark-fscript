mod body;
mod declarations;
pub mod error;

use fsc_vm_ir::Program;

use self::error::LowerResult;
use crate::ast::Script;

pub fn lower(script: &Script) -> LowerResult<Program> {
    let declared = declarations::collect(script)?;
    body::lower(script, declared)
}
