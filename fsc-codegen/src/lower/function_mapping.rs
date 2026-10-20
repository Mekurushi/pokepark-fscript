use crate::{CodegenError, CodegenResult};
use cranelift_entity::SecondaryMap;
use fsb_ir::{ExternalFunction, FunctionId as FsbFunctionId, FunctionRef, Program};
use fsc_mir::{FunctionId, FunctionKind, Linkage, Script};

pub(super) struct FunctionMapping {
    declarations: SecondaryMap<FunctionId, Option<FunctionRef>>,
}

impl FunctionMapping {
    pub(super) fn declare(script: &Script, program: &mut Program) -> Self {
        let mut declarations = SecondaryMap::new();

        for (source, function) in &script.functions {
            let reference = match &function.kind {
                FunctionKind::Defined { linkage, .. } => FunctionRef::Internal(
                    program.create_function(&function.name, lower_linkage(*linkage)),
                ),
                FunctionKind::External => {
                    FunctionRef::External(ExternalFunction::new(&function.name))
                }
            };
            declarations.insert(source, Some(reference));
        }

        Self { declarations }
    }

    pub(super) fn function(&self, source: FunctionId) -> CodegenResult<FunctionRef> {
        self.declarations
            .get(source)
            .and_then(Clone::clone)
            .ok_or(CodegenError::MissingFunctionMapping(source))
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
