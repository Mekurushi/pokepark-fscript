use crate::build::body;
use crate::{Function, FunctionId, FunctionKind, FunctionSignature, Linkage, Script, SourceInfo};
use cranelift_entity::{PrimaryMap, SecondaryMap};
use fsc_sema::thir;

pub fn lower(thir: &thir::Script) -> Script {
    LoweringContext::new(thir).lower_script()
}

pub(super) struct FunctionMapping {
    source_functions: PrimaryMap<FunctionId, thir::FunctionId>,
    lowered_functions: SecondaryMap<thir::FunctionId, Option<FunctionId>>,
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
    functions: FunctionMapping,
}

impl<'thir> LoweringContext<'thir> {
    fn new(thir: &'thir thir::Script) -> Self {
        Self {
            thir,
            functions: FunctionMapping::collect(thir),
        }
    }

    fn lower_script(&self) -> Script {
        let statics = PrimaryMap::new();
        let mut functions = PrimaryMap::new();

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
