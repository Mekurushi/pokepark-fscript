use crate::error::{AssemblerError, AssemblerResult};

const INSTRUCTION_LENGTH: i32 = 4;
const JUMP_OPCODE: u8 = 0x8;

pub fn encode_syscall(page: u8, func: u8, argc: u8, opcode: u8) -> u32 {
    u32::from_be_bytes([page, func, argc, opcode])
}
pub fn instruction_length() -> u32 {
    0x4 // TODO: real insn len logic
}
pub(crate) fn calculate_call_operand(
    current_offset: u32,
    target_offset: u32,
) -> AssemblerResult<i16> {
    let branch_offset =
        target_offset.cast_signed() - (current_offset.cast_signed() + INSTRUCTION_LENGTH);
    i16::try_from(branch_offset / INSTRUCTION_LENGTH)
        .map_err(|_err| AssemblerError::OperandOutOfRange(branch_offset))
}

// TODO: find clean way to make this reusable so assembler and this don't have his own versions
pub fn encode_relative_jump(source_offset: u32, target_offset: u32) -> AssemblerResult<[u8; 4]> {
    let operand = calculate_call_operand(source_offset, target_offset)?;
    Ok(InsnWord::new(JUMP_OPCODE)
        .operand(operand)
        .build()
        .to_be_bytes())
}

/// Builds a single 32-bit instruction word following the fscript slaspec.
///
/// Token layout:
///   opcode        = (0,7)
///   subtype       = (8,15)
///   sop           = (8,11)
///   `indirect_load` = (12,12)
///   operand       = (16,31) signed
///   `syscall_page`  = (26,31)
///   `syscall_func`  = (16,25)
///   imm           = (8,31)
///   simm          = (8,31) signed
#[must_use]
pub(crate) struct InsnWord(u32);

impl InsnWord {
    pub(crate) fn new(opcode: u8) -> Self {
        Self(u32::from(opcode))
    }

    /// subtype field — bits (8,15)
    pub(crate) fn subtype(mut self, val: u8) -> Self {
        self.0 |= u32::from(val) << 8;
        self
    }

    /// sop field — bits (8,11)
    pub(crate) fn sop(mut self, val: u8) -> Self {
        self.0 |= u32::from(val & 0xf) << 8;
        self
    }

    // TODO: rename? name bit misleading in my opinion
    /// `indirect_load` field — bit (12)
    pub(crate) fn indirect_load(mut self, val: bool) -> Self {
        self.0 |= u32::from(val) << 12;
        self
    }

    /// operand field — bits (16,31) signed
    pub(crate) fn operand(mut self, val: i16) -> Self {
        self.0 |= (u32::from(val.cast_unsigned())) << 16;
        self
    }

    /// `syscall_page` field — bits (26,31)
    pub(crate) fn syscall_page(mut self, val: u8) -> Self {
        self.0 |= u32::from(val & 0x3f) << 26;
        self
    }
    /// `syscall_func` field — bits (16,25)
    pub(crate) fn syscall_func(mut self, val: u16) -> Self {
        self.0 |= u32::from(val & 0x3ff) << 16;
        self
    }

    /// imm field — bits (8,31) unsigned
    pub(crate) fn imm(mut self, val: u32) -> Self {
        self.0 |= (val & 0x00ff_ffff) << 8;
        self
    }

    /// simm field — bits (8,31) signed
    #[allow(dead_code)]
    pub(crate) fn simm(mut self, val: i32) -> Self {
        self.0 |= (val.cast_unsigned() & 0x00ff_ffff) << 8;
        self
    }

    /// Produces the final encoded 32-bit instruction word
    pub(crate) fn build(self) -> u32 {
        self.0
    }
}
