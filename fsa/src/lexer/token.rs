use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum TokenKind {
    #[token("extern")]
    Extern,
    #[token("export")]
    Export,
    #[token("fn")]
    Fn,
    #[token("data")]
    Data,

    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("=")]
    Equal,
    #[token("-")]
    Minus,

    #[token("delay", |_| Mnemonic::Delay)]
    #[token("delay_load", |_| Mnemonic::DelayFromStack)]
    #[token("delay_neq0", |_| Mnemonic::DelayIfNonZero)]
    #[token("call", |_| Mnemonic::Call)]
    #[token("grow_stack", |_| Mnemonic::GrowStack)]
    #[token("shrink_stack", |_| Mnemonic::ShrinkStack)]
    #[token("load_arg", |_| Mnemonic::LoadFrameSlot)]
    #[token("load_arg_address", |_| Mnemonic::LoadFrameSlotAddress)]
    #[token("store_arg", |_| Mnemonic::StoreFrameSlot)]
    #[token("arg_addi", |_| Mnemonic::AddToFrameSlot)]
    #[token("arg_subi", |_| Mnemonic::SubtractFromFrameSlot)]
    #[token("load_rel", |_| Mnemonic::LoadRel)]
    #[token("push", |_| Mnemonic::PushConstant)]
    #[token("push_result", |_| Mnemonic::PushResult)]
    #[token("lstr", |_| Mnemonic::LoadString)]
    #[token("add", |_| Mnemonic::Add)]
    #[token("sub", |_| Mnemonic::Subtract)]
    #[token("mul", |_| Mnemonic::Multiply)]
    #[token("div", |_| Mnemonic::Divide)]
    #[token("mod", |_| Mnemonic::Remainder)]
    #[token("and", |_| Mnemonic::And)]
    #[token("or", |_| Mnemonic::Or)]
    #[token("xor", |_| Mnemonic::Xor)]
    #[token("not", |_| Mnemonic::Not)]
    #[token("eq0", |_| Mnemonic::IsZero)]
    #[token("neg", |_| Mnemonic::Negate)]
    #[token("fadd", |_| Mnemonic::FloatAdd)]
    #[token("fsub", |_| Mnemonic::FloatSubtract)]
    #[token("fmul", |_| Mnemonic::FloatMultiply)]
    #[token("fdiv", |_| Mnemonic::FloatDivide)]
    #[token("feq0", |_| Mnemonic::FloatIsZero)]
    #[token("fneg", |_| Mnemonic::FloatNegate)]
    #[token("eq", |_| Mnemonic::Equal)]
    #[token("neq", |_| Mnemonic::NotEqual)]
    #[token("lt", |_| Mnemonic::LessThan)]
    #[token("gt", |_| Mnemonic::GreaterThan)]
    #[token("le", |_| Mnemonic::LessOrEqual)]
    #[token("ge", |_| Mnemonic::GreaterOrEqual)]
    #[token("feq", |_| Mnemonic::FloatEqual)]
    #[token("fneq", |_| Mnemonic::FloatNotEqual)]
    #[token("flt", |_| Mnemonic::FloatLessThan)]
    #[token("fgt", |_| Mnemonic::FloatGreaterThan)]
    #[token("fle", |_| Mnemonic::FloatLessOrEqual)]
    #[token("fge", |_| Mnemonic::FloatGreaterOrEqual)]
    #[token("sl", |_| Mnemonic::ShiftLeft)]
    #[token("srm", |_| Mnemonic::ShiftRightArithmetic)]
    #[token("sr", |_| Mnemonic::ShiftRightLogical)]
    #[token("lea", |_| Mnemonic::LoadAddress)]
    #[token("lb", |_| Mnemonic::LoadByte)]
    #[token("ls", |_| Mnemonic::LoadHalfword)]
    #[token("lw", |_| Mnemonic::LoadWord)]
    #[token("lbi", |_| Mnemonic::LoadByteIndexed)]
    #[token("lsi", |_| Mnemonic::LoadHalfwordIndexed)]
    #[token("lwi", |_| Mnemonic::LoadWordIndexed)]
    #[token("sb", |_| Mnemonic::StoreByte)]
    #[token("ss", |_| Mnemonic::StoreHalfword)]
    #[token("sw", |_| Mnemonic::StoreWord)]
    #[token("sbi", |_| Mnemonic::StoreByteIndexed)]
    #[token("ssi", |_| Mnemonic::StoreHalfwordIndexed)]
    #[token("swi", |_| Mnemonic::StoreWordIndexed)]
    #[token("sbadd", |_| Mnemonic::AddByte)]
    #[token("ssadd", |_| Mnemonic::AddHalfword)]
    #[token("swadd", |_| Mnemonic::AddWord)]
    #[token("sbiadd", |_| Mnemonic::AddByteIndexed)]
    #[token("ssiadd", |_| Mnemonic::AddHalfwordIndexed)]
    #[token("swiadd", |_| Mnemonic::AddWordIndexed)]
    #[token("sbsub", |_| Mnemonic::SubtractByte)]
    #[token("sssub", |_| Mnemonic::SubtractHalfword)]
    #[token("swsub", |_| Mnemonic::SubtractWord)]
    #[token("sbisub", |_| Mnemonic::SubtractByteIndexed)]
    #[token("ssisub", |_| Mnemonic::SubtractHalfwordIndexed)]
    #[token("swisub", |_| Mnemonic::SubtractWordIndexed)]
    #[token("itof", |_| Mnemonic::IntegerToFloat)]
    #[token("ftoi", |_| Mnemonic::FloatToInteger)]
    #[token("debug", |_| Mnemonic::Debug)]
    #[token("jmp", |_| Mnemonic::Jump)]
    #[token("jz", |_| Mnemonic::JumpIfZero)]
    #[token("jz_pause", |_| Mnemonic::JumpIfZeroPause)]
    #[token("jz_set", |_| Mnemonic::JumpIfZeroSet)]
    #[token("jnz", |_| Mnemonic::JumpIfNonZero)]
    #[token("jnz_pause", |_| Mnemonic::JumpIfNonZeroPause)]
    #[token("jnz_set", |_| Mnemonic::JumpIfNonZeroSet)]
    #[token("jeq", |_| Mnemonic::JumpIfEqual)]
    #[token("jeq_imm", |_| Mnemonic::JumpIfEqualImmediate)]
    #[token("ret", |_| Mnemonic::Return)]
    #[token("retv", |_| Mnemonic::ReturnValue)]
    #[token("exit_1", |_| Mnemonic::Exit1)]
    #[token("exit_2", |_| Mnemonic::Exit2)]
    #[regex(r"SC[0-9]+", |lexer| {
        lexer.slice()[2..].parse::<u8>().ok().map(Mnemonic::Syscall)
    })]
    Mnemonic(Mnemonic),

    #[regex(r#"\"[^\"]*\""#, |lexer| {
        let text = lexer.slice();
        text[1..text.len() - 1].to_owned()
    })]
    String(String),

    #[regex(r"0[xX][0-9a-fA-F]+", |lexer| {
        i64::from_str_radix(&lexer.slice()[2..], 16).ok()
    })]
    #[regex(r"[0-9]+", |lexer| lexer.slice().parse::<i64>().ok())]
    Integer(i64), // i64 because it can hold both i32 and u32 temporarily and I don't want to do
    // the whole lexing and parsing stuff in 32-bit
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lexer| lexer.slice().to_owned())]
    Identifier(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {
    Syscall(u8),
    Delay,
    DelayFromStack,
    DelayIfNonZero,
    Call,
    GrowStack,
    ShrinkStack,
    LoadFrameSlot,
    LoadFrameSlotAddress,
    StoreFrameSlot,
    AddToFrameSlot,
    SubtractFromFrameSlot,
    LoadRel,
    PushConstant,
    PushResult,
    LoadString,
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
    FloatAdd,
    FloatSubtract,
    FloatMultiply,
    FloatDivide,
    FloatIsZero,
    FloatNegate,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
    FloatEqual,
    FloatNotEqual,
    FloatLessThan,
    FloatGreaterThan,
    FloatLessOrEqual,
    FloatGreaterOrEqual,
    ShiftLeft,
    ShiftRightArithmetic,
    ShiftRightLogical,
    LoadAddress,
    LoadByte,
    LoadHalfword,
    LoadWord,
    LoadByteIndexed,
    LoadHalfwordIndexed,
    LoadWordIndexed,
    StoreByte,
    StoreHalfword,
    StoreWord,
    StoreByteIndexed,
    StoreHalfwordIndexed,
    StoreWordIndexed,
    AddByte,
    AddHalfword,
    AddWord,
    AddByteIndexed,
    AddHalfwordIndexed,
    AddWordIndexed,
    SubtractByte,
    SubtractHalfword,
    SubtractWord,
    SubtractByteIndexed,
    SubtractHalfwordIndexed,
    SubtractWordIndexed,
    IntegerToFloat,
    FloatToInteger,
    Debug,
    Jump,
    JumpIfZero,
    JumpIfZeroPause,
    JumpIfZeroSet,
    JumpIfNonZero,
    JumpIfNonZeroPause,
    JumpIfNonZeroSet,
    JumpIfEqual,
    JumpIfEqualImmediate,
    Return,
    ReturnValue,
    Exit1,
    Exit2,
}
