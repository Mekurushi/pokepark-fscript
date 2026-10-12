use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionId(u32);

entity_impl!(FunctionId, "function");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalId(u32);

entity_impl!(LocalId, "local");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExprId(u32);

entity_impl!(ExprId, "expression");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FieldIdx(u32);

entity_impl!(FieldIdx, "field");
