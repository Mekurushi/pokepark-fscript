mod error;
mod frame;
mod lower;

pub use error::{CodegenError, CodegenResult};

pub fn lower(script: &fsc_mir::Script) -> CodegenResult<fsb_ir::Program> {
    lower::lower(script)
}
