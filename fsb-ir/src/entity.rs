use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionId(u32);

entity_impl!(FunctionId, "func");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DataId(u32);

entity_impl!(DataId, "data");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlockId(u32);

entity_impl!(BlockId, "block");
