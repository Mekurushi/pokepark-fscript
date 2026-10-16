#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Array(Box<[ConstValue]>),
    Aggregate(Box<[ConstValue]>),
}
