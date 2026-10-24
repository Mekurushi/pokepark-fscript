use super::body::BodyLowerer;
use crate::config::BoundConfigs;
use crate::const_eval::ConstEvalResults;
use crate::raw_syscall::RawSyscallResults;
use crate::typeck::TypeckResults;
use crate::{hir, thir};
use cranelift_entity::{PrimaryMap, SecondaryMap};

pub(crate) fn lower(
    hir: &hir::Script,
    typeck: &TypeckResults,
    configs: &BoundConfigs,
    constants: &ConstEvalResults,
    raw_syscalls: &RawSyscallResults,
) -> thir::Script {
    LoweringContext::new(hir, typeck, configs, constants, raw_syscalls).lower_script()
}

pub(super) struct FunctionMapping {
    items: PrimaryMap<thir::FunctionId, hir::ItemId>,
    functions: SecondaryMap<hir::ItemId, Option<thir::FunctionId>>,
}

pub(super) struct StaticMapping {
    items: PrimaryMap<thir::StaticId, hir::ItemId>,
    statics: SecondaryMap<hir::ItemId, Option<thir::StaticId>>,
}

impl StaticMapping {
    fn collect(script: &hir::Script) -> Self {
        let mut items = PrimaryMap::new();
        let mut statics = SecondaryMap::new();
        for (item, declaration) in &script.items {
            if matches!(declaration.kind, hir::ItemKind::Static(_)) {
                let static_var = items.push(item);
                statics.insert(item, Some(static_var));
            }
        }
        Self { items, statics }
    }

    pub(super) fn static_var(&self, item: hir::ItemId) -> Option<thir::StaticId> {
        self.statics.get(item).and_then(|static_var| *static_var)
    }
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
    pub(super) raw_syscalls: &'analysis RawSyscallResults,
    pub(super) statics: StaticMapping,
    pub(super) functions: FunctionMapping,
}

impl<'analysis> LoweringContext<'analysis> {
    fn new(
        hir: &'analysis hir::Script,
        typeck: &'analysis TypeckResults,
        configs: &'analysis BoundConfigs,
        constants: &'analysis ConstEvalResults,
        raw_syscalls: &'analysis RawSyscallResults,
    ) -> Self {
        Self {
            hir,
            typeck,
            configs,
            constants,
            raw_syscalls,
            statics: StaticMapping::collect(hir),
            functions: FunctionMapping::collect(hir),
        }
    }

    fn lower_script(&self) -> thir::Script {
        let mut statics = PrimaryMap::new();
        for (expected, item) in &self.statics.items {
            let actual = statics.push(self.lower_static(*item));
            debug_assert_eq!(actual, expected);
        }
        let mut functions = PrimaryMap::new();
        for (expected, item) in &self.functions.items {
            let actual = functions.push(self.lower_function(*item));
            debug_assert_eq!(actual, expected);
        }
        thir::Script { statics, functions }
    }

    fn lower_static(&self, item: hir::ItemId) -> thir::Static {
        let declaration = &self.hir.items[item];
        let hir::ItemKind::Static(static_var) = &declaration.kind else {
            unreachable!("non-static item reached THIR static lowering");
        };
        let kind = match &static_var.kind {
            hir::StaticKind::Defined { .. } => {
                let Some(initializer) = self.constants.static_initializer(item) else {
                    unreachable!("unevaluated static initializer reached THIR lowering");
                };
                thir::StaticKind::Defined {
                    initializer: lower_constant(initializer),
                }
            }
            hir::StaticKind::External => thir::StaticKind::External,
        };
        thir::Static {
            name: declaration.name.clone(),
            ty: static_var.ty.clone(),
            kind,
            source: thir::SourceInfo {
                span: declaration.name_span,
            },
        }
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

fn lower_constant(value: &crate::const_eval::ConstValue) -> thir::ConstValue {
    match value {
        crate::const_eval::ConstValue::Int(value) => thir::ConstValue::Int(*value),
        crate::const_eval::ConstValue::Float(value) => thir::ConstValue::Float(*value),
        crate::const_eval::ConstValue::Bool(value) => thir::ConstValue::Bool(*value),
        crate::const_eval::ConstValue::String(value) => thir::ConstValue::String(value.clone()),
        crate::const_eval::ConstValue::Array(elements) => {
            thir::ConstValue::Array(elements.iter().map(lower_constant).collect())
        }
        crate::const_eval::ConstValue::Aggregate(fields) => {
            thir::ConstValue::Aggregate(fields.iter().map(lower_constant).collect())
        }
    }
}
