mod config;
mod hir;
mod resolve;
#[allow(dead_code)]
mod typeck;
pub mod thir;
pub mod types;

pub use config::{ConfigRequirement, ConfigType, ConfigValue, ConfigValues};
pub use resolve::resolve;
