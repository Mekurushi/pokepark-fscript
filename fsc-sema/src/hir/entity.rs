use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ItemId(u32);

entity_impl!(ItemId, "item");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalId(u32);

entity_impl!(LocalId, "local");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExprId(u32);

entity_impl!(ExprId, "expression");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExprFieldId(u32);

entity_impl!(ExprFieldId, "expression field");
