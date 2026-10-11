// TODO: add verification phase.

// This is intended as an IR close to the target and kinda as an equivalent what LLVM IR would be
// in a regular compiler pipeline, even if not necessary for the size of this compiler I think it
// would make it easier to support pokepark2 in the future
mod control_flow;
mod entity;
mod instruction;
mod program;
mod word;

pub use control_flow::{BasicBlock, BranchKind, BranchMode, ExitKind, ReturnKind, Terminator};
pub use entity::{BlockId, DataId, FunctionId};
pub use instruction::{
    AddressingMode, Comparison, ConversionKind, DataRef, ExternalData, ExternalFunction,
    FloatOperation, FunctionRef, Instruction, IntegerOperation, MemoryWidth, ShiftOperation,
    StoreOperation,
};
pub use program::{Data, Function, Linkage, Program};
pub use word::Word;
