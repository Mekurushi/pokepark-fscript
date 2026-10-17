use super::error::BindError;
use super::{ConfigType, ConfigValue, ConfigValues};
use crate::hir;
use cranelift_entity::SecondaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub(crate) struct BoundConfigs {
    values: SecondaryMap<hir::ItemId, Option<ConfigValue>>,
}

impl BoundConfigs {
    pub(crate) fn value(&self, item: hir::ItemId) -> Option<&ConfigValue> {
        self.values.get(item).and_then(Option::as_ref)
    }
}

pub(crate) fn bind(
    script: &hir::Script,
    supplied: &ConfigValues,
    diagnostics: &mut Diagnostics,
) -> Option<BoundConfigs> {
    let declared_names = script
        .items
        .values()
        .filter(|item| matches!(item.kind, hir::ItemKind::Config(_)))
        .map(|item| item.name.as_str())
        .collect::<HashSet<_>>();
    let mut failed = false;

    for name in supplied.values.keys() {
        if !declared_names.contains(name.as_str()) {
            failed = true;
            diagnostics.emit_error(Diagnostic::from(BindError::UnknownConfig {
                name: name.clone(),
            }));
        }
    }

    let mut bound = BoundConfigs::default();
    for (item_id, item) in &script.items {
        let hir::ItemKind::Config(config) = &item.kind else {
            continue;
        };
        let Some(value) = supplied.values.get(&item.name) else {
            failed = true;
            diagnostics.emit_error(Diagnostic::from(BindError::MissingValue {
                name: item.name.clone(),
                declaration_span: item.name_span,
            }));
            continue;
        };
        let Some(expected) = ConfigType::from_ty(&config.ty) else {
            debug_assert!(false, "unsupported config type reached config binding");
            continue;
        };
        let actual = value.ty();
        if actual != expected {
            failed = true;
            diagnostics.emit_error(Diagnostic::from(BindError::TypeMismatch {
                name: item.name.clone(),
                expected,
                actual,
                declaration_span: config.type_span,
            }));
            continue;
        }
        bound.values.insert(item_id, Some(value.clone()));
    }

    if failed { None } else { Some(bound) }
}
