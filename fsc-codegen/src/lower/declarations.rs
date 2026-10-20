use super::function_mapping::FunctionMapping;
use crate::CodegenResult;
use crate::data::StaticData;
use fsb_ir::Program;
use fsc_mir::Script;

pub(super) struct DeclaredProgram {
    pub(super) program: Program,
    pub(super) statics: StaticData,
    pub(super) functions: FunctionMapping,
}

impl DeclaredProgram {
    pub(super) fn collect(script: &Script) -> CodegenResult<Self> {
        let mut program = Program::new();
        let statics = StaticData::declare(script, &mut program)?;
        let functions = FunctionMapping::declare(script, &mut program);

        Ok(Self {
            program,
            statics,
            functions,
        })
    }

    pub(super) fn static_data(&self, source: fsc_mir::StaticId) -> CodegenResult<fsb_ir::DataRef> {
        self.statics.reference(source)
    }
}
