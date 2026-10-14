use crate::ConfigValue;

// TODO: adding vec3 support with proper typing and passing typeck results in for FieldIdx
// resolution
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConstValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
}

impl From<&ConfigValue> for ConstValue {
    fn from(value: &ConfigValue) -> Self {
        match value {
            ConfigValue::Int(value) => Self::Int(*value),
            ConfigValue::Float(value) => Self::Float(*value),
            ConfigValue::Bool(value) => Self::Bool(*value),
            ConfigValue::String(value) => Self::String(value.clone()),
        }
    }
}
