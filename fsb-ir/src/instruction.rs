use crate::{DataId, FunctionId, Word};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Syscall {
        argument_count: u8,
        page: u8,
        function: u16,
    },
    Delay(i16),
    DelayFromStack,
    DelayIfNonZero,
    Call(FunctionRef),
    GrowStack(u16),
    ShrinkStack(u16),
    LoadFrameSlot(i16),
    LoadFrameSlotAddress(i16),
    StoreFrameSlot(i16),
    AddToFrameSlot(i16),
    SubtractFromFrameSlot(i16),
    LoadRel(i16),
    PushConstant(Word),
    PushResult,
    LoadString(String),
    Integer(IntegerOperation),
    Float(FloatOperation),
    IntegerCompare(Comparison),
    FloatCompare(Comparison),
    Shift(ShiftOperation),
    LoadAddress(DataRef),
    Load {
        width: MemoryWidth,
        addressing: AddressingMode,
    },
    Store {
        width: MemoryWidth,
        addressing: AddressingMode,
        operation: StoreOperation,
    },
    Convert {
        kind: ConversionKind,
        stack_offset: i16,
    },
    Debug {
        // TODO: do this one layer higher, direct forwarding for simplicity for now
        subtype: u8,
        operand: u16,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FunctionRef {
    Internal(FunctionId),
    External(ExternalFunction),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataRef {
    Internal(DataId),
    External(ExternalData),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExternalFunction(String);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExternalData(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegerOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    And,
    Or,
    Xor,
    Not,
    IsZero,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloatOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    IsZero,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShiftOperation {
    Left,
    RightArithmetic,
    RightLogical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWidth {
    Byte,
    Halfword,
    Word,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressingMode {
    Direct,
    Indexed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreOperation {
    Assign,
    Add,
    Subtract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionKind {
    IntegerToFloat,
    FloatToInteger,
}

impl ExternalFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

impl ExternalData {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}
