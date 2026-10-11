use super::resolution::Resolutions;
use super::symbols::{SymbolId, SymbolKind, SymbolTable};
use super::type_check::ExpressionTypes;
use crate::bind::{ConfigRequirement, ConfigType};
use fsc_parse::ast;

pub struct CheckedScript {
    pub(crate) script: ast::Script,
    pub(crate) symbols: SymbolTable,
    pub(crate) functions: Vec<CheckedFunction>,
}

impl CheckedScript {
    #[must_use]
    pub const fn ast(&self) -> &ast::Script {
        &self.script
    }

    #[must_use]
    pub fn config_requirements(&self) -> Vec<ConfigRequirement> {
        self.symbols
            .iter()
            .filter_map(|(_, symbol)| {
                if !matches!(symbol.kind, SymbolKind::Config) {
                    return None;
                }
                Some(ConfigRequirement {
                    name: symbol.name.clone(),
                    ty: ConfigType::from_ty(&symbol.ty)?,
                })
            })
            .collect()
    }
}

pub struct CheckedFunction {
    pub(crate) function: ast::FuncDef,
    pub(crate) symbol: SymbolId,
    pub(crate) resolutions: Resolutions,
    pub(crate) expression_types: ExpressionTypes,
}
