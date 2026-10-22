use super::{
    declarations::DeclaredProgram,
    function::lower as lower_function,
    function_mapping::{FunctionDeclaration, FunctionMapping},
};
use crate::{CodegenResult, data::StaticData, frame::FrameLayout};
use fsc_mir::{FunctionKind, Script};

pub(super) struct CodegenContext<'declarations> {
    statics: &'declarations StaticData,
    functions: &'declarations FunctionMapping,
}

impl<'declarations> CodegenContext<'declarations> {
    pub(super) const fn new(
        statics: &'declarations StaticData,
        functions: &'declarations FunctionMapping,
    ) -> Self {
        Self { statics, functions }
    }

    pub(super) const fn statics(&self) -> &StaticData {
        self.statics
    }

    pub(super) fn function(
        &self,
        source: fsc_mir::FunctionId,
    ) -> CodegenResult<fsb_ir::FunctionRef> {
        self.functions.function(source)
    }

    pub(super) fn function_declaration(
        &self,
        source: fsc_mir::FunctionId,
    ) -> CodegenResult<&FunctionDeclaration> {
        self.functions.declaration(source)
    }
}

pub(super) struct ScriptLowerer<'mir> {
    mir: &'mir Script,
    declarations: DeclaredProgram,
}

impl<'mir> ScriptLowerer<'mir> {
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

        let codegen = CodegenContext::new(&self.declarations.statics, &self.declarations.functions);
        for function in self.mir.functions.keys() {
            if let FunctionKind::Defined { body, .. } = &self.mir.functions[function].kind {
                let target = self.declarations.functions.defined_function(function)?;
                let declaration = self.declarations.functions.declaration(function)?;
                let frame = FrameLayout::plan(body, &declaration.abi().parameters)?;
                let output = self.declarations.program.function_mut(target);
                lower_function(body, frame, &codegen, output)?;
            }
        }

        Ok(self.declarations.program)
    }
}
