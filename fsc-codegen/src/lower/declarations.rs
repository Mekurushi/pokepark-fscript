use crate::{CodegenError, CodegenResult};
use cranelift_entity::SecondaryMap;
use fsb_ir::{ExternalFunction, FunctionRef, Program};
use fsc_mir::{FunctionId, FunctionKind, Linkage, Script};

pub(super) struct DeclaredProgram {
    pub(super) program: Program,
    functions: SecondaryMap<FunctionId, Option<FunctionRef>>,
}

impl DeclaredProgram {
    pub(super) fn collect(script: &Script) -> Self {
        let mut program = Program::new();
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

        Self { program, functions }
    }

    pub(super) fn function(&self, source: FunctionId) -> CodegenResult<FunctionRef> {
        self.functions
            .get(source)
            .and_then(Clone::clone)
            .ok_or(CodegenError::MissingFunctionMapping(source))
    }
}

const fn lower_linkage(linkage: Linkage) -> fsb_ir::Linkage {
    match linkage {
        Linkage::Internal => fsb_ir::Linkage::Internal,
        Linkage::Exported => fsb_ir::Linkage::Exported,
    }
}
