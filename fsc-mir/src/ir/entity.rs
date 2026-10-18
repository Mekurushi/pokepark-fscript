use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionId(u32);

entity_impl!(FunctionId, "function");

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalId(u32);

entity_impl!(LocalId, "local");

// it seems to be natural to collect implicit invariant the lower it semantically goes, I guess
// there is no
// better way; so I'm at least commenting them. Taking over the convention from rust where the
// return value is reserved as the first element and followed by argument_count of function
// arguments. That allows us to stronger decouple the return Terminator from the returned value
// which is more compatible with the lower semantics and makes it easier to write the result
// before returning
pub const RETURN_PLACE: LocalId = LocalId(0);

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BasicBlockId(u32);

entity_impl!(BasicBlockId, "block");

// similar to RETURN_PLACE, this is reserved for the start block of the function that should
// always be in the body
pub const START_BLOCK: BasicBlockId = BasicBlockId(0);
