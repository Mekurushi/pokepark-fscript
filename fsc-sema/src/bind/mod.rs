mod error;
mod value;

use crate::checked::CheckedScript;
use crate::symbol::{ConstValue, SymbolKind};
use std::collections::HashSet;

pub use error::{BindError, BindErrors};
pub use value::{ConfigRequirement, ConfigType, ConfigValue, ConfigValues};

pub struct BoundScript(pub(crate) CheckedScript);

pub fn bind_configs(
    mut checked: CheckedScript,
    supplied: &ConfigValues,
) -> Result<BoundScript, BindErrors> {
    let declared_names = checked
        .symbols
        .iter()
        .filter(|(_, symbol)| matches!(symbol.kind, SymbolKind::Config))
        .map(|(_, symbol)| symbol.name.as_str())
        .collect::<HashSet<_>>();
    let mut errors = supplied
        .values
        .keys()
        .filter(|name| !declared_names.contains(name.as_str()))
        .map(|name| BindError::UnknownConfig { name: name.clone() })
        .collect::<Vec<_>>();
    let mut values = Vec::new();

    for (symbol_id, config) in checked
        .symbols
        .iter()
        .filter(|(_, symbol)| matches!(symbol.kind, SymbolKind::Config))
    {
        let Some(value) = supplied.values.get(&config.name) else {
            errors.push(BindError::MissingValue {
                name: config.name.clone(),
                declaration_span: config.name_span,
            });
            continue;
        };
        let found = value.ty();
        if found != config.ty {
            errors.push(BindError::TypeMismatch {
                name: config.name.clone(),
                expected: config.ty.clone(),
                found,
                declaration_span: config.type_span,
            });
            continue;
        }
        let value = match value {
            ConfigValue::Int(value) => ConstValue::Int(*value),
            ConfigValue::Float(value) => ConstValue::Float(*value),
            ConfigValue::Bool(value) => ConstValue::Bool(*value),
            ConfigValue::String(value) => ConstValue::Str(value.clone()),
        };
        values.push((symbol_id, value));
    }

    if errors.is_empty() {
        for (symbol, value) in values {
            let value_span = checked.symbols.get(symbol).name_span;
            checked.symbols.get_mut(symbol).kind = SymbolKind::Const { value, value_span };
        }
        Ok(BoundScript(checked))
    } else {
        Err(BindErrors::new(errors))
    }
}
