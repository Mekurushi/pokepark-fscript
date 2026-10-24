use super::{
    cfg::CfgBuilder,
    context::{FunctionMapping, StaticMapping, lower_source},
    locals::LocalState,
    scope::ControlFlowScopes,
};
use crate::{Body, FunctionId, LocalId, StaticId};
use fsc_sema::{thir, types::Ty};

pub(super) struct BodyBuilder<'thir> {
    pub(super) thir: &'thir thir::Body,
    locals: LocalState,
    statics: &'thir StaticMapping,
    functions: &'thir FunctionMapping,
    function_source: thir::SourceInfo,
    pub(super) cfg: CfgBuilder,
    pub(super) scopes: ControlFlowScopes,
}

pub(super) fn lower(
    thir: &thir::Body,
    return_type: &Ty,
    function_source: thir::SourceInfo,
    statics: &StaticMapping,
    functions: &FunctionMapping,
) -> Body {
    let mut builder = BodyBuilder::new(thir, return_type, function_source, statics, functions);
    builder.lower_block(&thir.block);
    builder.finish()
}

impl<'thir> BodyBuilder<'thir> {
    fn new(
        thir: &'thir thir::Body,
        return_type: &Ty,
        function_source: thir::SourceInfo,
        statics: &'thir StaticMapping,
        functions: &'thir FunctionMapping,
    ) -> Self {
        Self {
            thir,
            locals: LocalState::lower(thir, return_type, function_source),
            statics,
            functions,
            function_source,
            cfg: CfgBuilder::new(),
            scopes: ControlFlowScopes::default(),
        }
    }

    fn finish(self) -> Body {
        let blocks = self.cfg.finish(lower_source(self.function_source));
        let (locals, variable_debug_info) = self.locals.into_parts();

        Body {
            argument_count: self.thir.parameter_locals.len(),
            locals,
            variable_debug_info,
            blocks,
        }
    }

    pub(super) fn mapped_local(&self, local: thir::LocalId) -> LocalId {
        self.locals.mapped(local)
    }

    pub(super) fn allocate_temporary(&mut self, ty: &Ty, source: thir::SourceInfo) -> LocalId {
        self.locals.allocate_temporary(ty, source)
    }

    pub(super) fn mapped_function(&self, function: thir::FunctionId) -> FunctionId {
        match self.functions.lowered(function) {
            Some(function) => function,
            None => unreachable!("every THIR function must have a MIR mapping"),
        }
    }

    pub(super) fn mapped_static(&self, static_var: thir::StaticId) -> StaticId {
        match self.statics.lowered(static_var) {
            Some(static_var) => static_var,
            None => unreachable!("every THIR static must have a MIR mapping"),
        }
    }
}
