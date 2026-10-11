use crate::assembler::Assembler;
use crate::assembly_unit::AssemblyUnit;
use crate::encoding::InsnWord;
use crate::error::AssemblerResult;
// opcodes
#[repr(u8)]
pub enum Opcode {
    SC = 0x1,
    Ctrl = 0x2,
    Call = 0x3,
    Return = 0x06,
    GrowStack = 0x07,
    Jump = 0x8,
    JeqImm = 0xa,
    LoadArg = 0x0b,
    ArgMem = 0x0c,
    LoadRel = 0x0d,
    ShrinkStack = 0xf,
    Push = 0x10,
    PushImm = 0x11,
    PushResult = 0x12,
    LStr = 0x13,
    Alu = 0x14,
    FAlu = 0x15,
    Cmp = 0x16,
    FCmp = 0x17,
    Shift = 0x18,
    Lea = 0x19,
    Load = 0x1A,
    Store = 0x1B,
    Conv = 0x1C,
    Debug = 0x1E,
}

#[repr(u8)]
pub enum CtrlSubtype {
    Delay = 0,
    // TODO: check and commit to the semantic naming Abort.
    Exit1 = 1,
    // TODO: check and commit to the semantic naming Reset.
    Exit2 = 2,
    DelayLoad = 3,
    DelayNeq0 = 4,
    LoadArgRef = 5,
}

#[repr(u8)]
pub enum ReturnSubtype {
    Ret = 0,
    Retv = 1,
}

#[repr(u8)]
pub enum JumpSubtype {
    Jmp = 0,
    Jnz = 1,
    Jz = 2,
    JnzPause = 3,
    JzPause = 4,
    JnzSet = 5,
    JzSet = 6,
    Jeq = 7,
}

#[repr(u8)]
pub enum ArgMemSubtype {
    StoreArg = 0,
    ArgAddi = 1,
    ArgSubi = 2,
}

#[repr(u16)]
pub enum ShiftOp {
    Sl = 0,
    Srm = 1,
    Sr = 2,
}

#[repr(u16)]
pub enum FaluOp {
    Fadd = 0,
    Fsub = 1,
    Fmul = 2,
    Fdiv = 3,
    Feq0 = 9,
    Fneg = 10,
}

#[repr(u16)]
pub enum CmpOp {
    Eq = 0xb,
    Neq = 0xc,
    Gt = 0xd,
    Lt = 0xe,
    Ge = 0xf,
    Le = 0x10,
}

#[repr(u16)]
pub enum FCmpOp {
    Feq = 0xb,
    Fneq = 0xc,
    Fgt = 0xd,
    Flt = 0xe,
    Fge = 0xf,
    Fle = 0x10,
}

#[repr(u16)]
pub enum AluOp {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
    Mod = 4,
    And = 5,
    Or = 6,
    Xor = 7,
    Not = 8,
    Eq0 = 9,
    Neg = 10,
}

#[repr(u8)]
pub enum StoreSubtype {
    Store = 0,
    Add = 1,
    Sub = 2,
}

#[repr(i16)]
pub enum LoadStoreSize {
    Byte = 1,
    Short = 2,
    Word = 4,
}

#[repr(u8)]
pub enum ConvSubtype {
    ItoF = 0,
    FtoI = 1,
}

pub(crate) enum RelocationKind {
    ExternalCall { symbol: String },
    ExternalDataAddress { symbol: String },
    StringOffset { value: String },
}

pub(crate) struct Relocation {
    pub(crate) code_offset: u32,
    pub(crate) kind: RelocationKind,
}

impl Assembler {
    pub fn emit_data_word(&mut self, value: u32) {
        self.emit(value);
    }

    // instruction emission

    // --- SC (0x1) ---
    pub fn emit_syscall(&mut self, subtype: u8, page: u8, func: u16) {
        self.emit(
            InsnWord::new(Opcode::SC as u8)
                .subtype(subtype)
                .syscall_page(page)
                .syscall_func(func)
                .build(),
        );
    }

    // --- Ctrl (0x2) ---

    pub fn emit_delay(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::Ctrl as u8)
                .subtype(CtrlSubtype::Delay as u8)
                .operand(operand)
                .build(),
        );
    }

    pub fn emit_exit_1(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Ctrl as u8)
                .subtype(CtrlSubtype::Exit1 as u8)
                .build(),
        );
    }
    pub fn emit_exit_2(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Ctrl as u8)
                .subtype(CtrlSubtype::Exit2 as u8)
                .build(),
        );
    }

    pub fn emit_delay_load(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Ctrl as u8)
                .subtype(CtrlSubtype::DelayLoad as u8)
                .build(),
        );
    }
    pub fn emit_delay_neq0(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Ctrl as u8)
                .subtype(CtrlSubtype::DelayNeq0 as u8)
                .build(),
        );
    }
    pub fn emit_load_arg_ref(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Ctrl as u8)
                .subtype(CtrlSubtype::LoadArgRef as u8)
                .build(),
        );
    }

    // --- Call (0x3) ---

    pub fn emit_call(&mut self, symbol: &str) -> AssemblerResult<()> {
        self.push_relocation(RelocationKind::ExternalCall {
            symbol: symbol.to_owned(),
        });
        self.emit(InsnWord::new(Opcode::Call as u8).build());
        Ok(())
    }

    pub(crate) fn emit_call_placeholder(&mut self) {
        self.emit(InsnWord::new(Opcode::Call as u8).build());
    }

    // --- Return (0x6) ---
    pub fn emit_ret(&mut self, n: i16) {
        // TODO: define explicit usage of operand; Ghidra visualizes as neagtive but actual operand is positive
        let operand = n.unsigned_abs().cast_signed();
        self.emit(
            InsnWord::new(Opcode::Return as u8)
                .subtype(ReturnSubtype::Ret as u8)
                .operand(operand)
                .build(),
        );
    }
    pub fn emit_retv(&mut self, n: i16) {
        let operand = n.unsigned_abs().cast_signed();
        self.emit(
            InsnWord::new(Opcode::Return as u8)
                .subtype(ReturnSubtype::Retv as u8)
                .operand(operand)
                .build(),
        );
    }
    // --- grow_stack (0x7) ---

    pub fn emit_grow_stack(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::GrowStack as u8)
                .operand(operand)
                .build(),
        );
    }

    // --- Jump (0x8) ---

    // --- load_arg (0xb) ---
    pub fn emit_load_arg(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::LoadArg as u8)
                .operand(operand)
                .build(),
        );
    }

    // --- ArgMem (0xc) ---
    pub fn emit_store_arg(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::ArgMem as u8)
                .subtype(ArgMemSubtype::StoreArg as u8)
                .operand(operand)
                .build(),
        );
    }

    pub fn emit_arg_addi(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::ArgMem as u8)
                .subtype(ArgMemSubtype::ArgAddi as u8)
                .operand(operand)
                .build(),
        );
    }
    pub fn emit_arg_subi(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::ArgMem as u8)
                .subtype(ArgMemSubtype::ArgSubi as u8)
                .operand(operand)
                .build(),
        );
    }

    pub fn emit_load_rel(&mut self, offset: i16) {
        self.emit(InsnWord::new(Opcode::LoadRel as u8).operand(offset).build());
    }

    // --- shrink_stack (0xf) ---

    pub fn emit_shrink_stack(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::ShrinkStack as u8)
                .operand(operand)
                .build(),
        );
    }

    // --- push (0x10) ---

    pub fn emit_push(&mut self, operand: i16) {
        self.emit(InsnWord::new(Opcode::Push as u8).operand(operand).build());
    }

    // --- push_imm (0x11) ---
    pub fn emit_push_imm(&mut self, operand: u32) {
        self.emit(InsnWord::new(Opcode::PushImm as u8).build());
        self.emit(operand);
    }
    // --- push_result (0x12) ---
    pub fn emit_push_result(&mut self) {
        self.emit(InsnWord::new(Opcode::PushResult as u8).build());
    }

    // --- lstr (0x13) ---
    pub fn emit_lstr(&mut self, s: &str) -> AssemblerResult<()> {
        self.string_table.intern(s)?;
        self.push_relocation(RelocationKind::StringOffset {
            value: s.to_owned(),
        });
        self.emit(InsnWord::new(Opcode::LStr as u8).build());

        Ok(())
    }

    // --- alu (0x14) ---

    pub fn emit_add(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Add as i16)
                .build(),
        );
    }
    pub fn emit_sub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Sub as i16)
                .build(),
        );
    }
    pub fn emit_mul(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Mul as i16)
                .build(),
        );
    }

    pub fn emit_div(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Div as i16)
                .build(),
        );
    }

    pub fn emit_mod(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Mod as i16)
                .build(),
        );
    }

    pub fn emit_and(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::And as i16)
                .build(),
        );
    }

    pub fn emit_or(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Or as i16)
                .build(),
        );
    }

    pub fn emit_xor(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Xor as i16)
                .build(),
        );
    }

    pub fn emit_not(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Not as i16)
                .build(),
        );
    }

    pub fn emit_eq0(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Eq0 as i16)
                .build(),
        );
    }
    pub fn emit_neg(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Alu as u8)
                .operand(AluOp::Neg as i16)
                .build(),
        );
    }

    // --- FAlu (0x15) ---

    pub fn emit_fadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FAlu as u8)
                .operand(FaluOp::Fadd as i16)
                .build(),
        );
    }
    pub fn emit_fsub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FAlu as u8)
                .operand(FaluOp::Fsub as i16)
                .build(),
        );
    }

    pub fn emit_fmul(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FAlu as u8)
                .operand(FaluOp::Fmul as i16)
                .build(),
        );
    }

    pub fn emit_fdiv(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FAlu as u8)
                .operand(FaluOp::Fdiv as i16)
                .build(),
        );
    }

    pub fn emit_feq0(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FAlu as u8)
                .operand(FaluOp::Feq0 as i16)
                .build(),
        );
    }

    pub fn emit_fneg(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FAlu as u8)
                .operand(FaluOp::Fneg as i16)
                .build(),
        );
    }

    // --- Cmp (0x16) ---
    pub fn emit_eq(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Cmp as u8)
                .operand(CmpOp::Eq as i16)
                .build(),
        );
    }
    pub fn emit_neq(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Cmp as u8)
                .operand(CmpOp::Neq as i16)
                .build(),
        );
    }

    pub fn emit_lt(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Cmp as u8)
                .operand(CmpOp::Lt as i16)
                .build(),
        );
    }

    pub fn emit_gt(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Cmp as u8)
                .operand(CmpOp::Gt as i16)
                .build(),
        );
    }

    pub fn emit_le(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Cmp as u8)
                .operand(CmpOp::Le as i16)
                .build(),
        );
    }

    pub fn emit_ge(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Cmp as u8)
                .operand(CmpOp::Ge as i16)
                .build(),
        );
    }

    // --- FCmp (0x17) ---
    pub fn emit_feq(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FCmp as u8)
                .operand(FCmpOp::Feq as i16)
                .build(),
        );
    }

    pub fn emit_fneq(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FCmp as u8)
                .operand(FCmpOp::Fneq as i16)
                .build(),
        );
    }
    pub fn emit_flt(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FCmp as u8)
                .operand(FCmpOp::Flt as i16)
                .build(),
        );
    }

    pub fn emit_fgt(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FCmp as u8)
                .operand(FCmpOp::Fgt as i16)
                .build(),
        );
    }
    pub fn emit_fle(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FCmp as u8)
                .operand(FCmpOp::Fle as i16)
                .build(),
        );
    }

    pub fn emit_fge(&mut self) {
        self.emit(
            InsnWord::new(Opcode::FCmp as u8)
                .operand(FCmpOp::Fge as i16)
                .build(),
        );
    }

    // --- shift (0x18) ---

    pub fn emit_sl(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Shift as u8)
                .operand(ShiftOp::Sl as i16)
                .build(),
        );
    }

    pub fn emit_srm(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Shift as u8)
                .operand(ShiftOp::Srm as i16)
                .build(),
        );
    }

    pub fn emit_sr(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Shift as u8)
                .operand(ShiftOp::Sr as i16)
                .build(),
        );
    }

    // --- lea (0x19) ---

    pub(crate) fn emit_lea_placeholder(&mut self) {
        self.emit(InsnWord::new(Opcode::Lea as u8).build());
    }

    pub(crate) fn emit_external_data_address(&mut self, symbol: &str) {
        self.push_relocation(RelocationKind::ExternalDataAddress {
            symbol: symbol.to_owned(),
        });
        self.emit_lea_placeholder();
    }

    // --- Load (0x1a) ---

    pub fn emit_lb(&mut self) {
        self.emit(InsnWord::new(Opcode::Load as u8).operand(1).build());
    }
    pub fn emit_ls(&mut self) {
        self.emit(InsnWord::new(Opcode::Load as u8).operand(2).build());
    }
    pub fn emit_lw(&mut self) {
        self.emit(InsnWord::new(Opcode::Load as u8).operand(4).build());
    }
    pub fn emit_lbi(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Load as u8)
                .operand(1)
                .indirect_load(true)
                .build(),
        );
    }
    pub fn emit_lsi(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Load as u8)
                .operand(2)
                .indirect_load(true)
                .build(),
        );
    }
    pub fn emit_lwi(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Load as u8)
                .operand(4)
                .indirect_load(true)
                .build(),
        );
    }

    // --- Store (0x1b) ---

    pub fn emit_sb(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Byte as i16)
                .sop(StoreSubtype::Store as u8)
                .build(),
        );
    }
    pub fn emit_ss(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Short as i16)
                .sop(StoreSubtype::Store as u8)
                .build(),
        );
    }
    pub fn emit_sw(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Word as i16)
                .sop(StoreSubtype::Store as u8)
                .build(),
        );
    }
    pub fn emit_sbi(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Byte as i16)
                .sop(StoreSubtype::Store as u8)
                .indirect_load(true)
                .build(),
        );
    }
    pub fn emit_ssi(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Short as i16)
                .sop(StoreSubtype::Store as u8)
                .indirect_load(true)
                .build(),
        );
    }
    pub fn emit_swi(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Word as i16)
                .sop(StoreSubtype::Store as u8)
                .indirect_load(true)
                .build(),
        );
    }

    pub fn emit_sbadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Byte as i16)
                .sop(StoreSubtype::Add as u8)
                .build(),
        );
    }
    pub fn emit_sbiadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Byte as i16)
                .sop(StoreSubtype::Add as u8)
                .indirect_load(true)
                .build(),
        );
    }

    pub fn emit_sbsub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Byte as i16)
                .sop(StoreSubtype::Sub as u8)
                .build(),
        );
    }
    pub fn emit_sbisub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Byte as i16)
                .sop(StoreSubtype::Sub as u8)
                .indirect_load(true)
                .build(),
        );
    }

    pub fn emit_ssadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Short as i16)
                .sop(StoreSubtype::Add as u8)
                .build(),
        );
    }
    pub fn emit_ssiadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Short as i16)
                .sop(StoreSubtype::Add as u8)
                .indirect_load(true)
                .build(),
        );
    }

    pub fn emit_sssub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Short as i16)
                .sop(StoreSubtype::Sub as u8)
                .build(),
        );
    }
    pub fn emit_ssisub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Short as i16)
                .sop(StoreSubtype::Sub as u8)
                .indirect_load(true)
                .build(),
        );
    }

    pub fn emit_swadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Word as i16)
                .sop(StoreSubtype::Add as u8)
                .build(),
        );
    }
    pub fn emit_swiadd(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Word as i16)
                .sop(StoreSubtype::Add as u8)
                .indirect_load(true)
                .build(),
        );
    }

    pub fn emit_swsub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Word as i16)
                .sop(StoreSubtype::Sub as u8)
                .build(),
        );
    }
    pub fn emit_swisub(&mut self) {
        self.emit(
            InsnWord::new(Opcode::Store as u8)
                .operand(LoadStoreSize::Word as i16)
                .sop(StoreSubtype::Sub as u8)
                .indirect_load(true)
                .build(),
        );
    }

    // --- Conv (0x1c) ---
    pub fn emit_itof(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::Conv as u8)
                .operand(operand)
                .subtype(ConvSubtype::ItoF as u8)
                .build(),
        );
    }

    pub fn emit_ftoi(&mut self, operand: i16) {
        self.emit(
            InsnWord::new(Opcode::Conv as u8)
                .operand(operand)
                .subtype(ConvSubtype::FtoI as u8)
                .build(),
        );
    }

    pub fn emit_debug(&mut self, subtype: u8, operand: u16) {
        self.emit(
            InsnWord::new(Opcode::Debug as u8)
                .subtype(subtype)
                .operand(operand.cast_signed())
                .build(),
        );
    }
    // --- helpers ---

    fn emit(&mut self, word: u32) {
        self.code.extend_from_slice(&word.to_be_bytes());
        self.program_counter += 4; // TODO: new Instruction Lenght way
    }

    pub(crate) const fn current_offset(&self) -> u32 {
        self.program_counter
    }

    pub(crate) fn emit_jump_placeholder(&mut self, subtype: JumpSubtype) {
        self.emit(
            InsnWord::new(Opcode::Jump as u8)
                .subtype(subtype as u8)
                .build(),
        );
    }

    pub(crate) fn emit_jeq_imm_placeholder(&mut self, immediate: u8) {
        self.emit(
            InsnWord::new(Opcode::JeqImm as u8)
                .subtype(immediate)
                .build(),
        );
    }

    fn push_relocation(&mut self, kind: RelocationKind) {
        self.relocations.push(Relocation {
            code_offset: self.program_counter,
            kind,
        });
    }

    pub fn into_unit(self) -> AssemblyUnit {
        AssemblyUnit::new(
            self.code,
            self.function_symbols,
            self.string_table,
            self.relocations,
        )
    }
}

#[cfg(test)]
mod emit_tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    use crate::assembler::Assembler;
    use crate::binary::FscriptBinary;

    // --- helpers ---
    fn assembler_in_function() -> Assembler {
        Assembler::new()
    }

    fn last_bytes(asm: &Assembler) -> [u8; 4] {
        let code = &asm.code;
        let idx = code.len() - 4;
        code[idx..idx + 4].try_into().unwrap()
    }

    fn insn_at(binary: &FscriptBinary, insn_index: usize) -> [u8; 4] {
        let offset = insn_index * 4;
        binary.code[offset..offset + 4].try_into().unwrap()
    }
    // --- SC ---

    #[test]
    fn emit_sc2_0x0_0x15() {
        let mut asm = assembler_in_function();
        asm.emit_syscall(2, 0x0, 0x15);
        assert_eq!(last_bytes(&asm), [0x00, 0x15, 0x02, 0x01]);
    }

    #[test]
    fn emit_sc3_0x0_0x15() {
        let mut asm = assembler_in_function();
        asm.emit_syscall(3, 0x0, 0x15);
        assert_eq!(last_bytes(&asm), [0x00, 0x15, 0x03, 0x01]);
    }

    #[test]
    fn emit_sc2_0x0_0x3() {
        let mut asm = assembler_in_function();
        asm.emit_syscall(2, 0x0, 0x3);
        assert_eq!(last_bytes(&asm), [0x00, 0x3, 0x02, 0x01]);
    }

    #[test]
    fn emit_sc1_0x0_0x1() {
        let mut asm = assembler_in_function();
        asm.emit_syscall(1, 0x0, 0x1);
        assert_eq!(last_bytes(&asm), [0x00, 0x1, 0x01, 0x01]);
    }

    #[test]
    fn emit_sc1_0x3_0x1() {
        let mut asm = assembler_in_function();
        asm.emit_syscall(1, 0x3, 0x1);
        assert_eq!(last_bytes(&asm), [0x0c, 0x1, 0x01, 0x01]);
    }

    // --- Ctrl ---
    #[test]
    fn emit_delay() {
        let mut asm = assembler_in_function();
        asm.emit_delay(5);
        assert_eq!(last_bytes(&asm), [0x00, 0x05, 0x00, 0x02]);
    }
    #[test]
    fn emit_delay_load() {
        let mut asm = assembler_in_function();
        asm.emit_delay_load();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x03, 0x02]);
    }

    #[test]
    fn emit_delay_neq0() {
        let mut asm = assembler_in_function();
        asm.emit_delay_neq0();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x04, 0x02]);
    }

    #[test]
    fn emit_exit1() {
        let mut asm = assembler_in_function();
        asm.emit_exit_1();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x01, 0x02]);
    }

    #[test]
    fn emit_exit2() {
        let mut asm = assembler_in_function();
        asm.emit_exit_2();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x02, 0x02]);
    }

    #[test]
    fn emit_load_arg_ref() {
        let mut asm = assembler_in_function();
        asm.emit_load_arg_ref();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x05, 0x02]);
    }

    // --- call ---

    #[test]
    fn emit_call_placeholder_bytes() {
        let mut asm = assembler_in_function();
        asm.emit_call("foo").unwrap();
        // operand=0 placeholder
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x03]);
    }

    // --- Return ---
    #[test]
    fn emit_ret_without_stack() {
        let mut asm = assembler_in_function();
        asm.emit_ret(0);

        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x6]);
    }
    #[test]
    fn emit_retv_without_stack() {
        let mut asm = assembler_in_function();
        asm.emit_retv(0);

        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x01, 0x06]);
    }
    #[test]
    fn emit_ret_with_stack() {
        let mut asm = assembler_in_function();
        asm.emit_ret(3);

        assert_eq!(last_bytes(&asm), [0x00, 0x03, 0x00, 0x6]);
    }
    #[test]
    fn emit_retv_with_stack() {
        let mut asm = assembler_in_function();
        asm.emit_retv(2);

        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x01, 0x06]);
    }
    // --- grow_stack ---
    #[test]
    fn emit_grow_stack() {
        let mut asm = assembler_in_function();
        asm.emit_grow_stack(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x07]);
    }

    // --- Jump ---

    // --- load_arg ---
    #[test]
    fn emit_load_arg_with_operand() {
        let mut asm = assembler_in_function();
        asm.emit_load_arg(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x0b]);
    }

    #[test]
    fn emit_load_arg_without_operand() {
        let mut asm = assembler_in_function();
        asm.emit_load_arg(0);
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x0b]);
    }

    // --- ArgMem ---
    #[test]
    fn emit_store_arg() {
        let mut asm = assembler_in_function();
        asm.emit_store_arg(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x0c]);
    }

    #[test]
    fn emit_arg_addi() {
        let mut asm = assembler_in_function();
        asm.emit_arg_addi(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x01, 0x0c]);
    }

    #[test]
    fn emit_arg_subi() {
        let mut asm = assembler_in_function();
        asm.emit_arg_subi(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x02, 0x0c]);
    }

    // --- shrink_stack ---

    #[test]
    fn emit_shrink_stack() {
        let mut asm = assembler_in_function();
        asm.emit_shrink_stack(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x0f]);
    }

    // --- push ---

    #[test]
    fn emit_push_1() {
        let mut asm = assembler_in_function();
        asm.emit_push(1);

        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x10]);
    }

    #[test]
    fn emit_push_negative1() {
        let mut asm = assembler_in_function();
        asm.emit_push(-1);

        assert_eq!(last_bytes(&asm), [0xff, 0xff, 0x00, 0x10]);
    }

    #[test]
    fn emit_push_100() {
        let mut asm = assembler_in_function();
        asm.emit_push(100);

        assert_eq!(last_bytes(&asm), [0x00, 0x64, 0x00, 0x10]);
    }

    // --- push_imm ---

    #[test]
    fn emit_push_imm() {
        let mut asm = assembler_in_function();
        asm.emit_push_imm(0x3f80_8000_u32);
        assert_eq!(&asm.code[0..4], &[0x00, 0x00, 0x00, 0x11]);
        assert_eq!(&asm.code[4..8], &[0x3f, 0x80, 0x80, 0x00]);
    }

    // --- push_result ---
    #[test]
    fn emit_push_result() {
        let mut asm = assembler_in_function();
        asm.emit_push_result();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x12]);
    }

    // --- lstr ---

    #[test]
    fn emit_lstr_first_string() {
        let mut asm = assembler_in_function();
        asm.emit_lstr("hello").unwrap();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x13]);
    }

    #[test]
    fn emit_lstr_second_string() {
        let mut asm = assembler_in_function();
        asm.emit_lstr("hello").unwrap(); // 5 bytes + null terminator
        asm.emit_lstr("hello2").unwrap();
        let binary = asm.into_unit().into_binary("test".to_string()).unwrap();
        assert_eq!(insn_at(&binary, 1), [0x00, 0x00, 0x06, 0x13]);
    }

    #[test]
    fn emit_lstr_same_string_returns_same_imm() {
        let mut asm = assembler_in_function();
        asm.emit_lstr("hello").unwrap(); // 5 bytes + null terminator
        asm.emit_lstr("hello").unwrap();
        let binary = asm.into_unit().into_binary("test".to_string()).unwrap();
        assert_eq!(insn_at(&binary, 1), [0x00, 0x00, 0x00, 0x13]);
    }

    #[test]
    fn emit_lstr_third_string() {
        let mut asm = assembler_in_function();
        asm.emit_lstr("hello").unwrap(); // 5 bytes + null terminator
        asm.emit_lstr("hello2").unwrap();
        asm.emit_lstr("hello3").unwrap();
        let binary = asm.into_unit().into_binary("test".to_string()).unwrap();
        assert_eq!(insn_at(&binary, 2), [0x00, 0x00, 0x0d, 0x13]);
    }

    // --- alu ---

    #[test]
    fn emit_add() {
        let mut asm = assembler_in_function();
        asm.emit_add();

        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x14]);
    }
    #[test]
    fn emit_sub() {
        let mut asm = assembler_in_function();
        asm.emit_sub();

        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x14]);
    }

    #[test]
    fn emit_mul() {
        let mut asm = assembler_in_function();
        asm.emit_mul();

        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x00, 0x14]);
    }

    #[test]
    fn emit_div() {
        let mut asm = assembler_in_function();
        asm.emit_div();

        assert_eq!(last_bytes(&asm), [0x00, 0x03, 0x00, 0x14]);
    }
    #[test]
    fn emit_mod() {
        let mut asm = assembler_in_function();
        asm.emit_mod();

        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x00, 0x14]);
    }

    #[test]
    fn emit_and() {
        let mut asm = assembler_in_function();
        asm.emit_and();

        assert_eq!(last_bytes(&asm), [0x00, 0x05, 0x00, 0x14]);
    }

    #[test]
    fn emit_or() {
        let mut asm = assembler_in_function();
        asm.emit_or();

        assert_eq!(last_bytes(&asm), [0x00, 0x06, 0x00, 0x14]);
    }

    #[test]
    fn emit_xor() {
        let mut asm = assembler_in_function();
        asm.emit_xor();

        assert_eq!(last_bytes(&asm), [0x00, 0x07, 0x00, 0x14]);
    }

    #[test]
    fn emit_not() {
        let mut asm = assembler_in_function();
        asm.emit_not();

        assert_eq!(last_bytes(&asm), [0x00, 0x08, 0x00, 0x14]);
    }

    #[test]
    fn emit_eq0() {
        let mut asm = assembler_in_function();
        asm.emit_eq0();
        assert_eq!(last_bytes(&asm), [0x00, 0x09, 0x00, 0x14]);
    }

    #[test]
    fn emit_neg() {
        let mut asm = assembler_in_function();
        asm.emit_neg();

        assert_eq!(last_bytes(&asm), [0x00, 0x0a, 0x00, 0x14]);
    }

    // --- falu ---
    #[test]
    fn emit_fadd() {
        let mut asm = assembler_in_function();
        asm.emit_fadd();

        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x15]);
    }

    #[test]
    fn emit_fsub() {
        let mut asm = assembler_in_function();
        asm.emit_fsub();

        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x15]);
    }

    #[test]
    fn emit_fmul() {
        let mut asm = assembler_in_function();
        asm.emit_fmul();

        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x00, 0x15]);
    }

    #[test]
    fn emit_fdiv() {
        let mut asm = assembler_in_function();
        asm.emit_fdiv();

        assert_eq!(last_bytes(&asm), [0x00, 0x03, 0x00, 0x15]);
    }

    #[test]
    fn emit_feq0() {
        let mut asm = assembler_in_function();
        asm.emit_feq0();

        assert_eq!(last_bytes(&asm), [0x00, 0x09, 0x00, 0x15]);
    }

    #[test]
    fn emit_fneg() {
        let mut asm = assembler_in_function();
        asm.emit_fneg();

        assert_eq!(last_bytes(&asm), [0x00, 0x0a, 0x00, 0x15]);
    }

    // --- Compare ---

    #[test]
    fn emit_eq() {
        let mut asm = assembler_in_function();
        asm.emit_eq();
        assert_eq!(last_bytes(&asm), [0x00, 0x0b, 0x00, 0x16]);
    }

    #[test]
    fn emit_neq() {
        let mut asm = assembler_in_function();
        asm.emit_neq();
        assert_eq!(last_bytes(&asm), [0x00, 0x0c, 0x00, 0x16]);
    }

    #[test]
    fn emit_lt() {
        let mut asm = assembler_in_function();
        asm.emit_lt();
        assert_eq!(last_bytes(&asm), [0x00, 0x0e, 0x00, 0x16]);
    }

    #[test]
    fn emit_gt() {
        let mut asm = assembler_in_function();
        asm.emit_gt();
        assert_eq!(last_bytes(&asm), [0x00, 0x0d, 0x00, 0x16]);
    }

    #[test]
    fn emit_le() {
        let mut asm = assembler_in_function();
        asm.emit_le();
        assert_eq!(last_bytes(&asm), [0x00, 0x10, 0x00, 0x16]);
    }

    #[test]
    fn emit_ge() {
        let mut asm = assembler_in_function();
        asm.emit_ge();
        assert_eq!(last_bytes(&asm), [0x00, 0x0f, 0x00, 0x16]);
    }

    // --- FCompare ---

    #[test]
    fn emit_feq() {
        let mut asm = assembler_in_function();
        asm.emit_feq();

        assert_eq!(last_bytes(&asm), [0x00, 0x0b, 0x00, 0x17]);
    }

    #[test]
    fn emit_fneq() {
        let mut asm = assembler_in_function();
        asm.emit_fneq();

        assert_eq!(last_bytes(&asm), [0x00, 0x0c, 0x00, 0x17]);
    }

    #[test]
    fn emit_flt() {
        let mut asm = assembler_in_function();
        asm.emit_flt();

        assert_eq!(last_bytes(&asm), [0x00, 0x0e, 0x00, 0x17]);
    }

    #[test]
    fn emit_fgt() {
        let mut asm = assembler_in_function();
        asm.emit_fgt();

        assert_eq!(last_bytes(&asm), [0x00, 0x0d, 0x00, 0x17]);
    }

    #[test]
    fn emit_fle() {
        let mut asm = assembler_in_function();
        asm.emit_fle();

        assert_eq!(last_bytes(&asm), [0x00, 0x10, 0x00, 0x17]);
    }

    #[test]
    fn emit_fge() {
        let mut asm = assembler_in_function();
        asm.emit_fge();

        assert_eq!(last_bytes(&asm), [0x00, 0x0f, 0x00, 0x17]);
    }

    // --- Shift ---
    #[test]
    fn emit_sl() {
        let mut asm = assembler_in_function();
        asm.emit_sl();
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x18]);
    }

    #[test]
    fn emit_srm() {
        let mut asm = assembler_in_function();
        asm.emit_srm();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x18]);
    }

    #[test]
    fn emit_sr() {
        let mut asm = assembler_in_function();
        asm.emit_sr();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x00, 0x18]);
    }

    // --- lea ---
    // TODO: clean implementation of lea; rethinking label symbol usage

    // --- Load ---

    #[test]
    fn emit_lb() {
        let mut asm = assembler_in_function();
        asm.emit_lb();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x1a]);
    }
    #[test]
    fn emit_ls() {
        let mut asm = assembler_in_function();
        asm.emit_ls();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x00, 0x1a]);
    }

    #[test]
    fn emit_lw() {
        let mut asm = assembler_in_function();
        asm.emit_lw();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x00, 0x1a]);
    }

    #[test]
    fn emit_lbi_bytes() {
        let mut asm = assembler_in_function();
        asm.emit_lbi();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x10, 0x1a]);
    }

    #[test]
    fn emit_lsi_bytes() {
        let mut asm = assembler_in_function();
        asm.emit_lsi();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x10, 0x1a]);
    }

    #[test]
    fn emit_lwi_bytes() {
        let mut asm = assembler_in_function();
        asm.emit_lwi();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x10, 0x1a]);
    }

    // --- Store ---

    #[test]
    fn emit_sb() {
        let mut asm = assembler_in_function();
        asm.emit_sb();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x1b]);
    }
    #[test]
    fn emit_ss() {
        let mut asm = assembler_in_function();
        asm.emit_ss();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x00, 0x1b]);
    }

    #[test]
    fn emit_sw() {
        let mut asm = assembler_in_function();
        asm.emit_sw();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x00, 0x1b]);
    }

    #[test]
    fn emit_sbi() {
        let mut asm = assembler_in_function();
        asm.emit_sbi();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x10, 0x1b]);
    }

    #[test]
    fn emit_ssi() {
        let mut asm = assembler_in_function();
        asm.emit_ssi();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x10, 0x1b]);
    }

    #[test]
    fn emit_swi() {
        let mut asm = assembler_in_function();
        asm.emit_swi();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x10, 0x1b]);
    }

    #[test]
    fn emit_sbadd() {
        let mut asm = assembler_in_function();
        asm.emit_sbadd();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x01, 0x1b]);
    }
    #[test]
    fn emit_sbiadd() {
        let mut asm = assembler_in_function();
        asm.emit_sbiadd();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x11, 0x1b]);
    }

    #[test]
    fn emit_sbsub() {
        let mut asm = assembler_in_function();
        asm.emit_sbsub();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x02, 0x1b]);
    }
    #[test]
    fn emit_sbisub() {
        let mut asm = assembler_in_function();
        asm.emit_sbisub();
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x12, 0x1b]);
    }

    #[test]
    fn emit_ssadd() {
        let mut asm = assembler_in_function();
        asm.emit_ssadd();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x01, 0x1b]);
    }
    #[test]
    fn emit_ssiadd() {
        let mut asm = assembler_in_function();
        asm.emit_ssiadd();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x11, 0x1b]);
    }

    #[test]
    fn emit_sssub() {
        let mut asm = assembler_in_function();
        asm.emit_sssub();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x02, 0x1b]);
    }
    #[test]
    fn emit_ssisub() {
        let mut asm = assembler_in_function();
        asm.emit_ssisub();
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x12, 0x1b]);
    }

    #[test]
    fn emit_swadd() {
        let mut asm = assembler_in_function();
        asm.emit_swadd();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x01, 0x1b]);
    }
    #[test]
    fn emit_swiadd() {
        let mut asm = assembler_in_function();
        asm.emit_swiadd();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x11, 0x1b]);
    }

    #[test]
    fn emit_swsub() {
        let mut asm = assembler_in_function();
        asm.emit_swsub();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x02, 0x1b]);
    }
    #[test]
    fn emit_swisub() {
        let mut asm = assembler_in_function();
        asm.emit_swisub();
        assert_eq!(last_bytes(&asm), [0x00, 0x04, 0x12, 0x1b]);
    }

    // --- Conv ---

    #[test]
    fn emit_itof_1() {
        let mut asm = assembler_in_function();
        asm.emit_itof(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x00, 0x1c]);
    }

    #[test]
    fn emit_itof_0() {
        let mut asm = assembler_in_function();
        asm.emit_itof(0);
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x00, 0x1c]);
    }

    #[test]
    fn emit_ftoi_1() {
        let mut asm = assembler_in_function();
        asm.emit_ftoi(1);
        assert_eq!(last_bytes(&asm), [0x00, 0x01, 0x01, 0x1c]);
    }

    #[test]
    fn emit_ftoi_0() {
        let mut asm = assembler_in_function();
        asm.emit_ftoi(0);
        assert_eq!(last_bytes(&asm), [0x00, 0x00, 0x01, 0x1c]);
    }

    #[test]
    fn emit_load_rel() {
        let mut asm = assembler_in_function();
        asm.emit_load_rel(2);
        assert_eq!(last_bytes(&asm), [0x00, 0x02, 0x00, 0x0d]);
    }

    #[test]
    fn emit_debug() {
        let mut asm = assembler_in_function();
        asm.emit_debug(0x12, 0x3456);
        assert_eq!(last_bytes(&asm), [0x34, 0x56, 0x12, 0x1e]);
    }
}
