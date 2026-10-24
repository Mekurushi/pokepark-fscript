use crate::build::body;
use crate::{
    Function, FunctionId, FunctionKind, FunctionSignature, Linkage, Script, SourceInfo, StaticDecl,
    StaticId, StaticKind,
};
use cranelift_entity::{PrimaryMap, SecondaryMap};
use fsc_sema::thir;

pub fn lower(thir: &thir::Script) -> Script {
    LoweringContext::new(thir).lower_script()
}

pub(super) struct FunctionMapping {
    source_functions: PrimaryMap<FunctionId, thir::FunctionId>,
    lowered_functions: SecondaryMap<thir::FunctionId, Option<FunctionId>>,
}

pub(super) struct StaticMapping {
    source_statics: PrimaryMap<StaticId, thir::StaticId>,
    lowered_statics: SecondaryMap<thir::StaticId, Option<StaticId>>,
}

impl StaticMapping {
    fn collect(script: &thir::Script) -> Self {
        let mut source_statics = PrimaryMap::new();
        let mut lowered_statics = SecondaryMap::new();

        for (source, _) in &script.statics {
            let lowered = source_statics.push(source);
            lowered_statics.insert(source, Some(lowered));
        }

        Self {
            source_statics,
            lowered_statics,
        }
    }

    pub(super) fn lowered(&self, source: thir::StaticId) -> Option<StaticId> {
        self.lowered_statics
            .get(source)
            .and_then(|static_var| *static_var)
    }
}

impl FunctionMapping {
    fn collect(script: &thir::Script) -> Self {
        let mut source_functions = PrimaryMap::new();
        let mut lowered_functions = SecondaryMap::new();

        for (source, _) in &script.functions {
            let lowered = source_functions.push(source);
            lowered_functions.insert(source, Some(lowered));
        }

        Self {
            source_functions,
            lowered_functions,
        }
    }

    pub(super) fn lowered(&self, source: thir::FunctionId) -> Option<FunctionId> {
        self.lowered_functions
            .get(source)
            .and_then(|function| *function)
    }
}

struct LoweringContext<'thir> {
    thir: &'thir thir::Script,
    statics: StaticMapping,
    functions: FunctionMapping,
}

impl<'thir> LoweringContext<'thir> {
    fn new(thir: &'thir thir::Script) -> Self {
        Self {
            thir,
            statics: StaticMapping::collect(thir),
            functions: FunctionMapping::collect(thir),
        }
    }

    fn lower_script(&self) -> Script {
        let mut statics = PrimaryMap::new();
        let mut functions = PrimaryMap::new();

        for (expected, source) in &self.statics.source_statics {
            debug_assert_eq!(self.statics.lowered(*source), Some(expected));
            let actual = statics.push(self.lower_static(*source));
            debug_assert_eq!(actual, expected);
        }

        for (expected, source) in &self.functions.source_functions {
            debug_assert_eq!(self.functions.lowered(*source), Some(expected));
            let actual = functions.push(self.lower_function(*source));
            debug_assert_eq!(actual, expected);
        }

        Script { statics, functions }
    }

    fn lower_function(&self, function: thir::FunctionId) -> Function {
        let source = &self.thir.functions[function];
        let signature = FunctionSignature {
            parameters: source.signature.parameters.clone(),
            return_type: source.signature.return_type.clone(),
        };
        let kind = match &source.kind {
            thir::FunctionKind::Defined { linkage, body } => FunctionKind::Defined {
                linkage: lower_linkage(*linkage),
                body: body::lower(
                    body,
                    &source.signature.return_type,
                    source.source,
                    &self.statics,
                    &self.functions,
                ),
            },
            thir::FunctionKind::External => FunctionKind::External,
        };

        Function {
            name: source.name.clone(),
            signature,
            kind,
            source: lower_source(source.source),
        }
    }

    fn lower_static(&self, static_var: thir::StaticId) -> StaticDecl {
        let source = &self.thir.statics[static_var];
        let kind = match &source.kind {
            thir::StaticKind::Defined { initializer } => StaticKind::Defined {
                initializer: lower_const(initializer),
            },
            thir::StaticKind::External => StaticKind::External,
        };

        StaticDecl {
            name: source.name.clone(),
            ty: source.ty.clone(),
            kind,
            source: lower_source(source.source),
        }
    }
}

fn lower_const(value: &thir::ConstValue) -> crate::ConstValue {
    match value {
        thir::ConstValue::Int(value) => crate::ConstValue::Int(*value),
        thir::ConstValue::Float(value) => crate::ConstValue::Float(*value),
        thir::ConstValue::Bool(value) => crate::ConstValue::Bool(*value),
        thir::ConstValue::String(value) => crate::ConstValue::String(value.clone()),
        thir::ConstValue::Array(elements) => {
            crate::ConstValue::Array(elements.iter().map(lower_const).collect())
        }
        thir::ConstValue::Aggregate(fields) => {
            crate::ConstValue::Aggregate(fields.iter().map(lower_const).collect())
        }
    }
}

const fn lower_linkage(linkage: thir::Linkage) -> Linkage {
    match linkage {
        thir::Linkage::Internal => Linkage::Internal,
        thir::Linkage::Exported => Linkage::Exported,
    }
}

pub(super) const fn lower_source(source: thir::SourceInfo) -> SourceInfo {
    SourceInfo { span: source.span }
}
