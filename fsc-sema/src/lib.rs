#[allow(dead_code)]
mod analysis;
mod config;
#[allow(dead_code)]
mod const_eval;
mod hir;
mod resolve;
mod thir;
#[allow(dead_code)]
mod typeck;
pub mod types;

pub use config::{ConfigRequirement, ConfigType, ConfigValue, ConfigValues};

use fsc_diagnostics::Diagnostics;

#[derive(Debug, Clone, Copy)]
pub enum CheckMode<'configs> {
    Source,
    Configured(&'configs ConfigValues),
}

pub fn check(
    script: &fsc_parse::ast::Script,
    mode: CheckMode<'_>,
) -> Diagnostics {
    match mode {
        CheckMode::Source => analysis::check(script),
        CheckMode::Configured(configs) => match analysis::analyze(script, configs) {
            Ok(analysis) => analysis.diagnostics,
            Err(diagnostics) => diagnostics,
        },
    }
}

pub fn required_configs(
    script: &fsc_parse::ast::Script,
) -> Result<Vec<ConfigRequirement>, Diagnostics> {
    analysis::requirements(script)
}
