use super::{body::BodyLowerer, declarations::DeclaredProgram};
use crate::{CodegenResult, frame::FrameLayout};
use fsc_mir::{FunctionKind, Script};

pub(super) struct LoweringContext<'mir> {
    mir: &'mir Script,
    declarations: DeclaredProgram,
}

impl<'mir> LoweringContext<'mir> {
    pub(super) fn new(mir: &'mir Script) -> CodegenResult<Self> {
        Ok(Self {
            mir,
            declarations: DeclaredProgram::collect(mir)?,
        })
    }

    pub(super) fn lower_script(mut self) -> CodegenResult<fsb_ir::Program> {
        for static_id in self.mir.statics.keys() {
            let _ = self.declarations.static_data(static_id)?;
        }
        for function in self.mir.functions.keys() {
            if let FunctionKind::Defined { body, .. } = &self.mir.functions[function].kind {
                let target = self.declarations.functions.defined_function(function)?;
                let frame = FrameLayout::plan(body)?;
                BodyLowerer::new(
                    body,
                    frame,
                    &self.declarations.statics,
                    &self.declarations.functions,
                    self.declarations.program.function_mut(target),
                )
                .lower()?;
            }
        }

        Ok(self.declarations.program)
    }
}
