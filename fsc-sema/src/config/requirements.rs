use super::{ConfigRequirement, ConfigType};
use crate::hir;

#[allow(dead_code)]
pub(crate) fn requirements(script: &hir::Script) -> Vec<ConfigRequirement> {
    script
        .items
        .values()
        .filter_map(|item| {
            let hir::ItemKind::Config(config) = &item.kind else {
                return None;
            };
            Some(ConfigRequirement {
                name: item.name.clone(),
                ty: ConfigType::from_ty(&config.ty)?,
            })
        })
        .collect()
}
