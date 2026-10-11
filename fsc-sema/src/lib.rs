mod bind;
mod check;
mod error;
pub mod thir;
pub mod types;

pub use bind::{
    BindError, BindErrors, BoundScript, ConfigRequirement, ConfigType, ConfigValue, ConfigValues,
    bind_configs,
};
pub use check::{CheckedFunction, CheckedScript, check};

pub fn lower(bound: BoundScript) -> error::SemaResult<thir::Script> {
    thir::lower_script(bound)
}
