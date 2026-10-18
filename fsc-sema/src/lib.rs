mod analysis;
mod config;
mod const_eval;
mod control_flow;
mod hir;
mod resolve;
pub mod thir;
mod typeck;
pub mod types;

pub use config::{ConfigRequirement, ConfigType, ConfigValue, ConfigValues};

use fsc_diagnostics::Diagnostics;

#[derive(Debug, Clone, Copy)]
pub enum CheckMode<'configs> {
    Source,
    Configured(&'configs ConfigValues),
}

#[derive(Debug)]
pub struct AnalyzeOutput {
    pub thir: thir::Script,
    pub diagnostics: Diagnostics,
}

pub fn check(script: &fsc_parse::ast::Script, mode: CheckMode<'_>) -> Diagnostics {
    match mode {
        CheckMode::Source => analysis::check(script),
        CheckMode::Configured(configs) => match analysis::analyze(script, configs) {
            Ok(analysis) => analysis.diagnostics,
            Err(diagnostics) => diagnostics,
        },
    }
}

pub fn analyze(
    script: &fsc_parse::ast::Script,
    configs: &ConfigValues,
) -> Result<AnalyzeOutput, Diagnostics> {
    let analysis = analysis::analyze(script, configs)?;
    let thir = thir::lower(
        &analysis.hir,
        &analysis.typeck,
        &analysis.configs,
        &analysis.const_eval,
    );

    Ok(AnalyzeOutput {
        thir,
        diagnostics: analysis.diagnostics,
    })
}

pub fn required_configs(
    script: &fsc_parse::ast::Script,
) -> Result<Vec<ConfigRequirement>, Diagnostics> {
    analysis::requirements(script)
}
