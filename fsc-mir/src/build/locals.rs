use super::context::lower_source;
use crate::{DebugValue, LocalDecl, LocalId, Place, RETURN_PLACE, VariableDebugInfo};
use cranelift_entity::{PrimaryMap, SecondaryMap};
use fsc_sema::{thir, types::Ty};

pub(super) struct LocalState {
    declarations: PrimaryMap<LocalId, LocalDecl>,
    mapping: SecondaryMap<thir::LocalId, Option<LocalId>>,
    debug_info: Vec<VariableDebugInfo>,
}

impl LocalState {
    pub(super) fn lower(
        thir: &thir::Body,
        return_type: &Ty,
        function_source: thir::SourceInfo,
    ) -> Self {
        let mut declarations = PrimaryMap::new();
        let mut mapping = SecondaryMap::new();
        let mut debug_info = Vec::new();

        // reserving the return place
        let return_local = declarations.push(LocalDecl {
            ty: return_type.clone(),
            source: lower_source(function_source),
        });
        debug_assert_eq!(return_local, RETURN_PLACE);

        // lowering parameters in signature orders
        for (index, source_local) in thir.parameter_locals.iter().copied().enumerate() {
            let declaration = &thir.locals[source_local];
            let lowered = declarations.push(LocalDecl {
                ty: declaration.ty.clone(),
                source: lower_source(declaration.source),
            });
            mapping.insert(source_local, Some(lowered));
            debug_info.push(lower_debug_info(declaration, lowered));
            debug_assert!(matches!(
                declaration.kind,
                thir::LocalKind::Parameter {
                    index: parameter_index
                } if parameter_index == index
            ));
        }

        // lowering remaining locals
        for (source_local, declaration) in &thir.locals {
            if matches!(declaration.kind, thir::LocalKind::Parameter { .. }) {
                continue;
            }

            let lowered = declarations.push(LocalDecl {
                ty: declaration.ty.clone(),
                source: lower_source(declaration.source),
            });
            mapping.insert(source_local, Some(lowered));
            debug_info.push(lower_debug_info(declaration, lowered));
        }

        // verifying that everything was mapped; ensuring the invariant holds
        for source_local in thir.locals.keys() {
            debug_assert!(
                mapping.get(source_local).and_then(|local| *local).is_some(),
                "THIR local was not mapped to MIR"
            );
        }

        Self {
            declarations,
            mapping,
            debug_info,
        }
    }

    pub(super) fn mapped(&self, local: thir::LocalId) -> LocalId {
        match self.mapping.get(local).and_then(|local| *local) {
            Some(local) => local,
            None => unreachable!("every THIR local must have a MIR mapping"),
        }
    }

    pub(super) fn allocate_temporary(&mut self, ty: &Ty, source: thir::SourceInfo) -> LocalId {
        self.declarations.push(LocalDecl {
            ty: ty.clone(),
            source: lower_source(source),
        })
    }

    pub(super) fn into_parts(self) -> (PrimaryMap<LocalId, LocalDecl>, Vec<VariableDebugInfo>) {
        (self.declarations, self.debug_info)
    }
}

fn lower_debug_info(local: &thir::Local, lowered: LocalId) -> VariableDebugInfo {
    VariableDebugInfo {
        name: local.name.clone(),
        value: DebugValue::Place(Place::from_local(lowered)),
        source: lower_source(local.source),
    }
}
