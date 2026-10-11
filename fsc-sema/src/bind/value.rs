use crate::types::Ty;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigType {
    Int,
    Float,
    Bool,
    String,
}

impl ConfigType {
    pub(crate) fn from_ty(ty: &Ty) -> Option<Self> {
        match ty {
            Ty::Int => Some(Self::Int),
            Ty::Float => Some(Self::Float),
            Ty::Bool => Some(Self::Bool),
            Ty::Str => Some(Self::String),
            Ty::Void | Ty::Vec3 => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRequirement {
    pub name: String,
    pub ty: ConfigType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
}

impl ConfigValue {
    pub(crate) fn ty(&self) -> Ty {
        match self {
            Self::Int(_) => Ty::Int,
            Self::Float(_) => Ty::Float,
            Self::Bool(_) => Ty::Bool,
            Self::String(_) => Ty::Str,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigValues {
    pub(crate) values: BTreeMap<String, ConfigValue>,
}

impl ConfigValues {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, value: ConfigValue) -> Option<ConfigValue> {
        self.values.insert(name.into(), value)
    }
}
