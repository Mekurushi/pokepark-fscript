#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Function(Function),
    ExternalFunction(String),
    Data(Data),
    ExternalData(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub linkage: Linkage,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    Internal,
    Exported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    pub name: String,
    pub words: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Label(String),
    Instruction(Instruction),
    Terminator(Terminator),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Syscall {
        argument_count: u8,
        page: u8,
        function: u16,
    },
    Delay(i16),
    DelayFromStack,
    DelayIfNonZero,
    Call(String),
    GrowStack(u16),
    ShrinkStack(u16),
    LoadFrameSlot(i16),
    LoadFrameSlotAddress(i16),
    StoreFrameSlot(i16),
    AddToFrameSlot(i16),
    SubtractFromFrameSlot(i16),
    LoadRel(i16),
    PushConstant(u32),
    PushResult,
    LoadString(String),
    Integer(IntegerOperation),
    Float(FloatOperation),
    IntegerCompare(Comparison),
    FloatCompare(Comparison),
    Shift(ShiftOperation),
    LoadAddress(String),
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
        subtype: u8,
        operand: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    Jump(String),
    Branch { kind: BranchKind, target: String },
    Return { kind: ReturnKind, frame_slots: u16 },
    Exit1,
    Exit2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    IsZero,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftOperation {
    Left,
    RightArithmetic,
    RightLogical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryWidth {
    Byte,
    Halfword,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    Direct,
    Indexed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOperation {
    Assign,
    Add,
    Subtract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    IntegerToFloat,
    FloatToInteger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchKind {
    IfZero(BranchMode),
    IfNonZero(BranchMode),
    IfEqual,
    IfEqualImmediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchMode {
    Normal,
    Pause,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnKind {
    None,
    Value,
}
