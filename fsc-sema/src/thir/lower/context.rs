use super::body::BodyLowerer;
use crate::config::BoundConfigs;
use crate::const_eval::ConstEvalResults;
use crate::typeck::TypeckResults;
use crate::{hir, thir};
use cranelift_entity::{PrimaryMap, SecondaryMap};

pub(crate) fn lower(
    hir: &hir::Script,
    typeck: &TypeckResults,
    configs: &BoundConfigs,
    constants: &ConstEvalResults,
) -> thir::Script {
    LoweringContext::new(hir, typeck, configs, constants).lower_script()
}

pub(super) struct FunctionMapping {
    items: PrimaryMap<thir::FunctionId, hir::ItemId>,
    functions: SecondaryMap<hir::ItemId, Option<thir::FunctionId>>,
}

impl FunctionMapping {
    fn collect(script: &hir::Script) -> Self {
        let mut items = PrimaryMap::new();
        let mut functions = SecondaryMap::new();
        for (item, declaration) in &script.items {
            if matches!(declaration.kind, hir::ItemKind::Function(_)) {
                let function = items.push(item);
                functions.insert(item, Some(function));
            }
        }
        Self { items, functions }
    }

    pub(super) fn function(&self, item: hir::ItemId) -> Option<thir::FunctionId> {
        self.functions.get(item).and_then(|function| *function)
    }
}

pub(super) struct LoweringContext<'analysis> {
    pub(super) hir: &'analysis hir::Script,
    pub(super) typeck: &'analysis TypeckResults,
    pub(super) configs: &'analysis BoundConfigs,
    pub(super) constants: &'analysis ConstEvalResults,
    pub(super) functions: FunctionMapping,
}

impl<'analysis> LoweringContext<'analysis> {
    fn new(
        hir: &'analysis hir::Script,
        typeck: &'analysis TypeckResults,
        configs: &'analysis BoundConfigs,
        constants: &'analysis ConstEvalResults,
    ) -> Self {
        Self {
            hir,
            typeck,
            configs,
            constants,
            functions: FunctionMapping::collect(hir),
        }
    }

    fn lower_script(&self) -> thir::Script {
        let mut functions = PrimaryMap::new();
        for (expected, item) in &self.functions.items {
            let actual = functions.push(self.lower_function(*item));
            debug_assert_eq!(actual, expected);
        }
        thir::Script { functions }
    }

    fn lower_function(&self, item: hir::ItemId) -> thir::Function {
        let declaration = &self.hir.items[item];
        let hir::ItemKind::Function(function) = &declaration.kind else {
            unreachable!("non-function item reached THIR function lowering");
        };
        let signature = thir::FunctionSignature {
            parameters: function
                .signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            return_type: function.signature.return_type.clone(),
        };
        let kind = match &function.kind {
            hir::FunctionKind::Defined { exported, body } => thir::FunctionKind::Defined {
                linkage: if *exported {
                    thir::Linkage::Exported
                } else {
                    thir::Linkage::Internal
                },
                body: self.lower_body(item, body),
            },
            hir::FunctionKind::External => thir::FunctionKind::External,
        };
        thir::Function {
            name: declaration.name.clone(),
            signature,
            kind,
            source: thir::SourceInfo {
                span: declaration.name_span,
            },
        }
    }

    fn lower_body(&self, owner: hir::ItemId, body: &hir::Body) -> thir::Body {
        let mut lowerer = BodyLowerer::new(self, owner, body);
        let block = lowerer.lower_block(&body.block);
        lowerer.finish(block)
    }
}
