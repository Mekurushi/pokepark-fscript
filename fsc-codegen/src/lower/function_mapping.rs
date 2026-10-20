use crate::{
    CodegenError, CodegenResult,
    abi::{PassMode, pass_mode},
};
use cranelift_entity::PrimaryMap;
use fsb_ir::{ExternalFunction, FunctionId as FsbFunctionId, FunctionRef, Program};
use fsc_mir::{FunctionId, FunctionKind, Linkage, Script};

pub(super) struct FunctionDeclaration {
    reference: FunctionRef,
    parameter_modes: Box<[PassMode]>,
}

impl FunctionDeclaration {
    pub(super) const fn parameter_modes(&self) -> &[PassMode] {
        &self.parameter_modes
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
            let parameter_modes = function
                .signature
                .parameters
                .iter()
                .map(|ty| {
                    pass_mode(ty).unwrap_or_else(|| {
                        unreachable!(
                            "unsupported parameter type reached codegen declaration \
                        lowering"
                        )
                    })
                })
                .collect();
            let declared = declarations.push(FunctionDeclaration {
                reference,
                parameter_modes,
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
