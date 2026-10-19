use crate::data::StaticData;
use crate::{CodegenError, CodegenResult};
use cranelift_entity::SecondaryMap;
use fsb_ir::{ExternalFunction, FunctionId as FsbFunctionId, FunctionRef, Program};
use fsc_mir::{FunctionId, FunctionKind, Linkage, Script};

pub(super) struct DeclaredProgram {
    pub(super) program: Program,
    pub(super) statics: StaticData,
    functions: SecondaryMap<FunctionId, Option<FunctionRef>>,
}

impl DeclaredProgram {
    pub(super) fn collect(script: &Script) -> CodegenResult<Self> {
        let mut program = Program::new();
        let statics = StaticData::declare(script, &mut program)?;
        let mut functions = SecondaryMap::new();

        for (source, function) in &script.functions {
            let reference = match &function.kind {
                FunctionKind::Defined { linkage, .. } => FunctionRef::Internal(
                    program.create_function(&function.name, lower_linkage(*linkage)),
                ),
                FunctionKind::External => {
                    FunctionRef::External(ExternalFunction::new(&function.name))
                }
            };
            functions.insert(source, Some(reference));
        }

        Ok(Self {
            program,
            statics,
            functions,
        })
    }

    pub(super) fn function(&self, source: FunctionId) -> CodegenResult<FunctionRef> {
        self.functions
            .get(source)
            .and_then(Clone::clone)
            .ok_or(CodegenError::MissingFunctionMapping(source))
    }

    pub(super) fn static_data(&self, source: fsc_mir::StaticId) -> CodegenResult<fsb_ir::DataRef> {
        self.statics.reference(source)
    }

    pub(super) fn defined_function(&self, source: FunctionId) -> CodegenResult<FsbFunctionId> {
        match self.function(source)? {
            FunctionRef::Internal(function) => Ok(function),
            FunctionRef::External(_) => Err(CodegenError::InvalidFunctionMapping(source)),
        }
    }
}

const fn lower_linkage(linkage: Linkage) -> fsb_ir::Linkage {
    match linkage {
        Linkage::Internal => fsb_ir::Linkage::Internal,
        Linkage::Exported => fsb_ir::Linkage::Exported,
    }
}
