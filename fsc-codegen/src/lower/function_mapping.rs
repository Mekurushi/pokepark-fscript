use crate::{CodegenError, CodegenResult, abi::FunctionAbi};
use cranelift_entity::PrimaryMap;
use fsb_ir::{ExternalFunction, FunctionId as FsbFunctionId, FunctionRef, Program};
use fsc_mir::{FunctionId, FunctionKind, Linkage, Script};

pub(super) struct FunctionDeclaration {
    reference: FunctionRef,
    abi: FunctionAbi,
}

impl FunctionDeclaration {
    pub(super) const fn abi(&self) -> &FunctionAbi {
        &self.abi
    }
}

pub(super) struct FunctionMapping {
    declarations: PrimaryMap<FunctionId, FunctionDeclaration>,
}

impl FunctionMapping {
    pub(super) fn declare(script: &Script, program: &mut Program) -> Self {
        let mut declarations = PrimaryMap::new();

        for (source, function) in &script.functions {
            let reference = match &function.kind {
                FunctionKind::Defined { linkage, .. } => FunctionRef::Internal(
                    program.create_function(&function.name, lower_linkage(*linkage)),
                ),
                FunctionKind::External => {
                    FunctionRef::External(ExternalFunction::new(&function.name))
                }
            };
            let abi = FunctionAbi::from_signature(&function.signature);
            let declared = declarations.push(FunctionDeclaration {
                reference,
                abi,
            });
            debug_assert_eq!(source, declared);
        }

        Self { declarations }
    }

    pub(super) fn declaration(&self, source: FunctionId) -> CodegenResult<&FunctionDeclaration> {
        self.declarations
            .get(source)
            .ok_or(CodegenError::MissingFunctionMapping(source))
    }

    pub(super) fn function(&self, source: FunctionId) -> CodegenResult<FunctionRef> {
        self.declaration(source)
            .map(|declaration| declaration.reference.clone())
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
