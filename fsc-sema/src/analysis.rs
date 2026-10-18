use crate::config::{self, BoundConfigs, ConfigRequirement, ConfigValues};
use crate::const_eval::{self, ConstEvalResults};
use crate::control_flow;
use crate::hir;
use crate::resolve;
use crate::typeck::{self, TypeckResults};
use fsc_diagnostics::Diagnostics;
use fsc_parse::ast;

pub(crate) struct Analysis {
    pub(crate) hir: hir::Script,
    pub(crate) typeck: TypeckResults,
    pub(crate) configs: BoundConfigs,
    pub(crate) const_eval: ConstEvalResults,
    pub(crate) diagnostics: Diagnostics,
}

struct FrontendAnalysis {
    hir: hir::Script,
    typeck: Option<TypeckResults>,
    had_errors: bool,
    diagnostics: Diagnostics,
}

pub(crate) fn check(script: &ast::Script) -> Diagnostics {
    analyze_frontend(script).diagnostics
}

pub(crate) fn requirements(script: &ast::Script) -> Result<Vec<ConfigRequirement>, Diagnostics> {
    let analysis = analyze_frontend(script);
    if analysis.had_errors {
        Err(analysis.diagnostics)
    } else {
        Ok(config::requirements(&analysis.hir))
    }
}

pub(crate) fn analyze(
    script: &ast::Script,
    supplied_configs: &ConfigValues,
) -> Result<Analysis, Diagnostics> {
    let FrontendAnalysis {
        hir,
        typeck,
        had_errors,
        mut diagnostics,
    } = analyze_frontend(script);

    let configs = config::bind(&hir, supplied_configs, &mut diagnostics);

    let (Some(typeck), Some(configs)) = (typeck, configs) else {
        return Err(diagnostics);
    };
    if had_errors {
        return Err(diagnostics);
    }

    let Some(const_eval) = const_eval::evaluate(&hir, &configs, &mut diagnostics) else {
        return Err(diagnostics);
    };
    Ok(Analysis {
        hir,
        typeck,
        configs,
        const_eval,
        diagnostics,
    })
}

fn analyze_frontend(script: &ast::Script) -> FrontendAnalysis {
    let mut diagnostics = Diagnostics::default();
    let resolve::ResolveOutput { hir, had_errors } = resolve::resolve(script, &mut diagnostics);
    let typeck = typeck::check(&hir, &mut diagnostics);
    let control_flow_errors = control_flow::check(&hir, &mut diagnostics);
    let had_errors = had_errors || typeck.is_none() || control_flow_errors;

    FrontendAnalysis {
        hir,
        typeck,
        had_errors,
        diagnostics,
    }
}
