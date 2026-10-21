use std::collections::HashMap;

use fsb_ir::{
    AddressingMode, BasicBlock, BlockId, BranchKind, BranchMode, Comparison, ConversionKind,
    DataId, DataRef, ExitKind, FloatOperation, FunctionId, FunctionRef, Instruction,
    IntegerOperation, Linkage, MemoryWidth, Program, ReturnKind, ShiftOperation, StoreOperation,
    Terminator,
};

use crate::AssemblyUnit;
use crate::emission::{JumpSubtype, Relocation};
use crate::encoding::calculate_call_operand;
use crate::error::{AssemblerError, AssemblerResult};
use crate::function_symbols::FunctionSymbols;
use crate::string_table::StringTable;

pub struct AssembledProgram {
    unit: AssemblyUnit,
    function_offsets: HashMap<FunctionId, u32>,
    exported_functions: HashMap<FunctionId, String>,
}

impl AssembledProgram {
    pub fn function_offset(&self, function: FunctionId) -> Option<u32> {
        self.function_offsets.get(&function).copied()
    }

    pub fn unit(&self) -> &AssemblyUnit {
        &self.unit
    }

    pub fn make_function_private(&mut self, function: FunctionId) -> AssemblerResult<()> {
        if !self.function_offsets.contains_key(&function) {
            return Err(AssemblerError::UndefinedSymbol(function.to_string()));
        }

        if let Some(name) = self.exported_functions.remove(&function) {
            self.unit.function_symbols.remove(&name)?;
        }

        Ok(())
    }

    pub fn into_unit(self) -> AssemblyUnit {
        self.unit
    }
}

pub fn assemble_program(program: &Program) -> AssemblerResult<AssembledProgram> {
    Assembler::new().assemble(program)
}

pub(super) struct Assembler {
    pub(super) code: Vec<u8>,
    pub(super) function_symbols: FunctionSymbols,
    pub(super) string_table: StringTable,
    pub(super) relocations: Vec<Relocation>,
    pub(super) program_counter: u32,
}

impl Assembler {
    pub(super) fn new() -> Self {
        Self {
            code: Vec::new(),
            function_symbols: FunctionSymbols::new(),
            string_table: StringTable::new(),
            relocations: Vec::new(),
            program_counter: 0,
        }
    }

    fn assemble(mut self, program: &Program) -> AssemblerResult<AssembledProgram> {
        let mut layout = Layout::default();
        let mut fixups = Vec::new();

        for (function_id, function) in program.functions() {
            layout.functions.insert(function_id, self.current_offset());

            for (block_id, block) in function.blocks() {
                layout
                    .blocks
                    .insert((function_id, block_id), self.current_offset());
                emit_block(
                    &mut self,
                    &mut fixups,
                    function_id,
                    function.name(),
                    block_id,
                    block,
                )?;
            }
        }

        for (id, data) in program.data_objects() {
            layout.data.insert(id, self.current_offset());
            for word in data.words() {
                self.emit_data_word(word.bits());
            }
        }

        let mut exported_functions = HashMap::new();
        for (id, function) in program.functions() {
            if function.linkage() == Linkage::Exported {
                exported_functions.insert(id, function.name().to_owned());
            }
        }
        let mut unit = self.into_unit();
        apply_fixups(&mut unit, &layout, &fixups)?;
        for (&id, name) in &exported_functions {
            unit.function_symbols
                .define_function(name.clone(), layout.function(id)?)?;
        }
        Ok(AssembledProgram {
            unit,
            function_offsets: layout.functions,
            exported_functions,
        })
    }
}

#[derive(Default)]
struct Layout {
    functions: HashMap<FunctionId, u32>,
    blocks: HashMap<(FunctionId, BlockId), u32>,
    data: HashMap<DataId, u32>,
}

impl Layout {
    fn function(&self, id: FunctionId) -> AssemblerResult<u32> {
        self.functions
            .get(&id)
            .copied()
            .ok_or_else(|| AssemblerError::UndefinedSymbol(id.to_string()))
    }

    fn block(&self, function: FunctionId, block: BlockId) -> AssemblerResult<u32> {
        self.blocks
            .get(&(function, block))
            .copied()
            .ok_or_else(|| AssemblerError::UndefinedSymbol(block.to_string()))
    }

    fn data(&self, id: DataId) -> AssemblerResult<u32> {
        self.data
            .get(&id)
            .copied()
            .ok_or_else(|| AssemblerError::UndefinedSymbol(id.to_string()))
    }
}
// fixup and relocations are split between the assembler (everything that can resolved in one
// unit) and the assembly unit that is responsible for the linking phase e.g. merging two units
// together and doing the relocation. Data is currently one unit local only, we would need to
// keep some metadata when we want to support "external" data references
struct Fixup {
    code_offset: u32,
    kind: FixupKind,
}

enum FixupKind {
    Call(FunctionId),
    Address(DataId),
    Jump {
        function: FunctionId,
        block: BlockId,
    },
}

fn apply_fixups(unit: &mut AssemblyUnit, layout: &Layout, fixups: &[Fixup]) -> AssemblerResult<()> {
    for fixup in fixups {
        let target_offset = match fixup.kind {
            FixupKind::Call(function) => layout.function(function)?,
            FixupKind::Address(data) => layout.data(data)?,
            FixupKind::Jump { function, block } => layout.block(function, block)?,
        };
        let operand = calculate_call_operand(fixup.code_offset, target_offset)?;
        let index = fixup.code_offset as usize;
        unit.code[index..index + 2].copy_from_slice(&operand.to_be_bytes());
    }
    Ok(())
}

fn emit_block(
    assembler: &mut Assembler,
    fixups: &mut Vec<Fixup>,
    function_id: FunctionId,
    function_name: &str,
    block_id: BlockId,
    block: &BasicBlock,
) -> AssemblerResult<()> {
    for instruction in block.instructions() {
        emit_instruction(assembler, fixups, instruction)?;
    }

    let terminator = block
        .terminator()
        .ok_or_else(|| AssemblerError::MissingTerminator {
            function: function_name.to_owned(),
            block: block_id.to_string(),
        })?;
    emit_terminator(assembler, fixups, function_id, terminator);
    Ok(())
}

fn emit_instruction(
    assembler: &mut Assembler,
    fixups: &mut Vec<Fixup>,
    instruction: &Instruction,
) -> AssemblerResult<()> {
    match instruction {
        Instruction::Syscall {
            argument_count,
            page,
            function,
        } => assembler.emit_syscall(*argument_count, *page, *function),
        Instruction::Delay(value) => assembler.emit_delay(*value),
        Instruction::DelayFromStack => assembler.emit_delay_load(),
        Instruction::DelayIfNonZero => assembler.emit_delay_neq0(),
        Instruction::Call(FunctionRef::Internal(id)) => {
            fixups.push(Fixup {
                code_offset: assembler.current_offset(),
                kind: FixupKind::Call(*id),
            });
            assembler.emit_call_placeholder();
        }
        Instruction::Call(FunctionRef::External(function)) => {
            assembler.emit_call(function.name())?;
        }
        Instruction::GrowStack(slots) => assembler.emit_grow_stack(slots.cast_signed()),
        Instruction::ShrinkStack(slots) => assembler.emit_shrink_stack(slots.cast_signed()),
        Instruction::LoadFrameSlot(offset) => assembler.emit_load_arg(*offset),
        Instruction::LoadFrameSlotAddress(offset) => {
            assembler.emit_load_arg_ref();
            assembler.emit_load_arg(*offset);
        }
        Instruction::StoreFrameSlot(offset) => assembler.emit_store_arg(*offset),
        Instruction::AddToFrameSlot(offset) => assembler.emit_arg_addi(*offset),
        Instruction::SubtractFromFrameSlot(offset) => assembler.emit_arg_subi(*offset),
        Instruction::LoadRel(offset) => assembler.emit_load_rel(*offset),
        Instruction::PushConstant(word) => {
            let value = word.bits().cast_signed();
            if let Ok(short) = i16::try_from(value) {
                assembler.emit_push(short);
            } else {
                assembler.emit_push_imm(word.bits());
            }
        }
        Instruction::PushResult => assembler.emit_push_result(),
        Instruction::LoadString(value) => assembler.emit_lstr(value)?,
        Instruction::Integer(operation) => emit_integer(assembler, *operation),
        Instruction::Float(operation) => emit_float(assembler, *operation),
        Instruction::IntegerCompare(comparison) => emit_integer_comparison(assembler, *comparison),
        Instruction::FloatCompare(comparison) => emit_float_comparison(assembler, *comparison),
        Instruction::Shift(operation) => emit_shift(assembler, *operation),
        Instruction::LoadAddress(DataRef::Internal(id)) => {
            fixups.push(Fixup {
                code_offset: assembler.current_offset(),
                kind: FixupKind::Address(*id),
            });
            assembler.emit_lea_placeholder();
        }
        Instruction::LoadAddress(DataRef::External(data)) => {
            assembler.emit_external_data_address(data.name());
        }
        Instruction::Load { width, addressing } => emit_load(assembler, *width, *addressing),
        Instruction::Store {
            width,
            addressing,
            operation,
        } => emit_store(assembler, *width, *addressing, *operation),
        Instruction::Convert { kind, stack_offset } => match kind {
            ConversionKind::IntegerToFloat => assembler.emit_itof(*stack_offset),
            ConversionKind::FloatToInteger => assembler.emit_ftoi(*stack_offset),
        },
        Instruction::Debug { subtype, operand } => assembler.emit_debug(*subtype, *operand),
    }

    Ok(())
}

fn emit_terminator(
    assembler: &mut Assembler,
    fixups: &mut Vec<Fixup>,
    function: FunctionId,
    terminator: &Terminator,
) {
    match terminator {
        Terminator::Jump(target) => {
            emit_jump(assembler, fixups, function, *target, JumpSubtype::Jmp);
        }
        Terminator::Branch {
            kind,
            taken,
            not_taken,
        } => {
            match kind {
                BranchKind::IfZero(BranchMode::Normal) => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::Jz);
                }
                BranchKind::IfZero(BranchMode::Pause) => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::JzPause);
                }
                BranchKind::IfZero(BranchMode::Set) => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::JzSet);
                }
                BranchKind::IfNonZero(BranchMode::Normal) => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::Jnz);
                }
                BranchKind::IfNonZero(BranchMode::Pause) => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::JnzPause);
                }
                BranchKind::IfNonZero(BranchMode::Set) => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::JnzSet);
                }
                BranchKind::IfEqual => {
                    emit_jump(assembler, fixups, function, *taken, JumpSubtype::Jeq);
                }
                BranchKind::IfEqualImmediate(immediate) => {
                    fixups.push(Fixup {
                        code_offset: assembler.current_offset(),
                        kind: FixupKind::Jump {
                            function,
                            block: *taken,
                        },
                    });
                    assembler.emit_jeq_imm_placeholder(*immediate);
                }
            }
            emit_jump(assembler, fixups, function, *not_taken, JumpSubtype::Jmp);
        }
        Terminator::Return { kind, frame_slots } => match kind {
            ReturnKind::None => assembler.emit_ret(frame_slots.cast_signed()),
            ReturnKind::Value => assembler.emit_retv(frame_slots.cast_signed()),
        },
        Terminator::Exit(ExitKind::Abort) => assembler.emit_exit_1(),
        Terminator::Exit(ExitKind::Reset) => assembler.emit_exit_2(),
    }
}

fn emit_jump(
    assembler: &mut Assembler,
    fixups: &mut Vec<Fixup>,
    function: FunctionId,
    block: BlockId,
    subtype: JumpSubtype,
) {
    fixups.push(Fixup {
        code_offset: assembler.current_offset(),
        kind: FixupKind::Jump { function, block },
    });
    assembler.emit_jump_placeholder(subtype);
}

fn emit_integer(assembler: &mut Assembler, operation: IntegerOperation) {
    match operation {
        IntegerOperation::Add => assembler.emit_add(),
        IntegerOperation::Subtract => assembler.emit_sub(),
        IntegerOperation::Multiply => assembler.emit_mul(),
        IntegerOperation::Divide => assembler.emit_div(),
        IntegerOperation::Remainder => assembler.emit_mod(),
        IntegerOperation::And => assembler.emit_and(),
        IntegerOperation::Or => assembler.emit_or(),
        IntegerOperation::Xor => assembler.emit_xor(),
        IntegerOperation::Not => assembler.emit_not(),
        IntegerOperation::IsZero => assembler.emit_eq0(),
        IntegerOperation::Negate => assembler.emit_neg(),
    }
}

fn emit_float(assembler: &mut Assembler, operation: FloatOperation) {
    match operation {
        FloatOperation::Add => assembler.emit_fadd(),
        FloatOperation::Subtract => assembler.emit_fsub(),
        FloatOperation::Multiply => assembler.emit_fmul(),
        FloatOperation::Divide => assembler.emit_fdiv(),
        FloatOperation::IsZero => assembler.emit_feq0(),
        FloatOperation::Negate => assembler.emit_fneg(),
    }
}

fn emit_integer_comparison(assembler: &mut Assembler, comparison: Comparison) {
    match comparison {
        Comparison::Equal => assembler.emit_eq(),
        Comparison::NotEqual => assembler.emit_neq(),
        Comparison::LessThan => assembler.emit_lt(),
        Comparison::GreaterThan => assembler.emit_gt(),
        Comparison::LessOrEqual => assembler.emit_le(),
        Comparison::GreaterOrEqual => assembler.emit_ge(),
    }
}

fn emit_float_comparison(assembler: &mut Assembler, comparison: Comparison) {
    match comparison {
        Comparison::Equal => assembler.emit_feq(),
        Comparison::NotEqual => assembler.emit_fneq(),
        Comparison::LessThan => assembler.emit_flt(),
        Comparison::GreaterThan => assembler.emit_fgt(),
        Comparison::LessOrEqual => assembler.emit_fle(),
        Comparison::GreaterOrEqual => assembler.emit_fge(),
    }
}

fn emit_shift(assembler: &mut Assembler, operation: ShiftOperation) {
    match operation {
        ShiftOperation::Left => assembler.emit_sl(),
        ShiftOperation::RightArithmetic => assembler.emit_srm(),
        ShiftOperation::RightLogical => assembler.emit_sr(),
    }
}

fn emit_load(assembler: &mut Assembler, width: MemoryWidth, addressing: AddressingMode) {
    match (width, addressing) {
        (MemoryWidth::Byte, AddressingMode::Direct) => assembler.emit_lb(),
        (MemoryWidth::Halfword, AddressingMode::Direct) => assembler.emit_ls(),
        (MemoryWidth::Word, AddressingMode::Direct) => assembler.emit_lw(),
        (MemoryWidth::Byte, AddressingMode::Indexed) => assembler.emit_lbi(),
        (MemoryWidth::Halfword, AddressingMode::Indexed) => assembler.emit_lsi(),
        (MemoryWidth::Word, AddressingMode::Indexed) => assembler.emit_lwi(),
    }
}

fn emit_store(
    assembler: &mut Assembler,
    width: MemoryWidth,
    addressing: AddressingMode,
    operation: StoreOperation,
) {
    match (width, addressing, operation) {
        (MemoryWidth::Byte, AddressingMode::Direct, StoreOperation::Assign) => assembler.emit_sb(),
        (MemoryWidth::Halfword, AddressingMode::Direct, StoreOperation::Assign) => {
            assembler.emit_ss();
        }
        (MemoryWidth::Word, AddressingMode::Direct, StoreOperation::Assign) => assembler.emit_sw(),
        (MemoryWidth::Byte, AddressingMode::Indexed, StoreOperation::Assign) => {
            assembler.emit_sbi();
        }
        (MemoryWidth::Halfword, AddressingMode::Indexed, StoreOperation::Assign) => {
            assembler.emit_ssi();
        }
        (MemoryWidth::Word, AddressingMode::Indexed, StoreOperation::Assign) => {
            assembler.emit_swi();
        }
        (MemoryWidth::Byte, AddressingMode::Direct, StoreOperation::Add) => assembler.emit_sbadd(),
        (MemoryWidth::Halfword, AddressingMode::Direct, StoreOperation::Add) => {
            assembler.emit_ssadd();
        }
        (MemoryWidth::Word, AddressingMode::Direct, StoreOperation::Add) => assembler.emit_swadd(),
        (MemoryWidth::Byte, AddressingMode::Indexed, StoreOperation::Add) => {
            assembler.emit_sbiadd();
        }
        (MemoryWidth::Halfword, AddressingMode::Indexed, StoreOperation::Add) => {
            assembler.emit_ssiadd();
        }
        (MemoryWidth::Word, AddressingMode::Indexed, StoreOperation::Add) => {
            assembler.emit_swiadd();
        }
        (MemoryWidth::Byte, AddressingMode::Direct, StoreOperation::Subtract) => {
            assembler.emit_sbsub();
        }
        (MemoryWidth::Halfword, AddressingMode::Direct, StoreOperation::Subtract) => {
            assembler.emit_sssub();
        }
        (MemoryWidth::Word, AddressingMode::Direct, StoreOperation::Subtract) => {
            assembler.emit_swsub();
        }
        (MemoryWidth::Byte, AddressingMode::Indexed, StoreOperation::Subtract) => {
            assembler.emit_sbisub();
        }
        (MemoryWidth::Halfword, AddressingMode::Indexed, StoreOperation::Subtract) => {
            assembler.emit_ssisub();
        }
        (MemoryWidth::Word, AddressingMode::Indexed, StoreOperation::Subtract) => {
            assembler.emit_swisub();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::similar_names)]
    use fsb_ir::{
        DataRef, ExternalData, ExternalFunction, FunctionRef, Instruction, Linkage, Program,
        ReturnKind, Terminator, Word,
    };

    use super::assemble_program;
    use crate::binary::FscriptBinary;
    use crate::{ExternalDataOffsets, ExternalFunctionOffsets};

    #[test]
    fn resolves_internal_calls_and_block_targets() {
        let mut program = Program::new();
        let caller = program.create_function("caller", Linkage::Exported);
        let callee = program.create_function("callee", Linkage::Internal);

        let entry = program.function_mut(caller).create_block();
        let continuation = program.function_mut(caller).create_block();
        program
            .function_mut(caller)
            .block_mut(entry)
            .push_instruction(Instruction::Call(FunctionRef::Internal(callee)));
        program
            .function_mut(caller)
            .block_mut(entry)
            .set_terminator(Terminator::Jump(continuation));
        program
            .function_mut(caller)
            .block_mut(continuation)
            .set_terminator(Terminator::Return {
                kind: ReturnKind::None,
                frame_slots: 0,
            });

        let callee_entry = program.function_mut(callee).create_block();
        program
            .function_mut(callee)
            .block_mut(callee_entry)
            .set_terminator(Terminator::Return {
                kind: ReturnKind::None,
                frame_slots: 0,
            });

        let unit = assemble_program(&program).unwrap().into_unit();

        assert_eq!(unit.function_offset("caller"), Some(0));
        assert_eq!(unit.function_offset("callee"), None);
        assert_eq!(unit.code[0..4], [0x00, 0x02, 0x00, 0x03]);
        assert_eq!(unit.code[4..8], [0x00, 0x00, 0x00, 0x08]);
    }

    #[test]
    fn resolves_data_addresses_after_function_code() {
        let mut program = Program::new();
        let data = program.create_data(None, vec![Word::from_bits(0x1234_5678)]);
        let function = program.create_function("main", Linkage::Exported);
        let entry = program.function_mut(function).create_block();
        program
            .function_mut(function)
            .block_mut(entry)
            .push_instruction(Instruction::LoadAddress(DataRef::Internal(data)));
        program
            .function_mut(function)
            .block_mut(entry)
            .set_terminator(Terminator::Return {
                kind: ReturnKind::None,
                frame_slots: 0,
            });

        let unit = assemble_program(&program).unwrap().into_unit();

        assert_eq!(unit.code[0..4], [0x00, 0x01, 0x00, 0x19]);
        assert_eq!(unit.code[8..12], 0x1234_5678_u32.to_be_bytes());
    }

    #[test]
    fn finalizes_external_calls_and_string_offsets() {
        let mut program = Program::new();
        let function = program.create_function("main", Linkage::Exported);
        let entry = program.function_mut(function).create_block();
        let block = program.function_mut(function).block_mut(entry);
        block.push_instruction(Instruction::Call(FunctionRef::External(
            ExternalFunction::new("external"),
        )));
        block.push_instruction(Instruction::LoadString("hello".to_owned()));
        block.set_terminator(Terminator::Return {
            kind: ReturnKind::None,
            frame_slots: 0,
        });

        let unit = assemble_program(&program).unwrap().into_unit();
        let mut external_functions = ExternalFunctionOffsets::new();
        external_functions.define("external", 12).unwrap();
        let serialized = unit
            .into_binary_with_externals(
                "test".to_owned(),
                &external_functions,
                &ExternalDataOffsets::new(),
            )
            .unwrap()
            .serialize()
            .unwrap();
        let binary = FscriptBinary::deserialize(&serialized).unwrap();
        let (_, unit) = crate::AssemblyUnit::from_binary(binary).unwrap();

        assert_eq!(unit.code[0..4], [0x00, 0x02, 0x00, 0x03]);
        assert_eq!(unit.code[4..8], [0x00, 0x00, 0x00, 0x13]);
    }

    #[test]
    fn finalizes_external_data_addresses() {
        let mut program = Program::new();
        let function = program.create_function("main", Linkage::Exported);
        let entry = program.function_mut(function).create_block();
        let block = program.function_mut(function).block_mut(entry);
        block.push_instruction(Instruction::LoadAddress(DataRef::External(
            ExternalData::new("external_data"),
        )));
        block.set_terminator(Terminator::Return {
            kind: ReturnKind::None,
            frame_slots: 0,
        });

        let unit = assemble_program(&program).unwrap().into_unit();
        let mut external_data = ExternalDataOffsets::new();
        external_data.define("external_data", 8).unwrap();
        let binary = unit
            .into_binary_with_externals(
                "test".to_owned(),
                &ExternalFunctionOffsets::new(),
                &external_data,
            )
            .unwrap();

        assert_eq!(binary.code[0..4], [0x00, 0x01, 0x00, 0x19]);
    }

    #[test]
    fn emits_only_exported_functions_to_the_binary_symbol_table() {
        let mut program = Program::new();
        for (name, linkage) in [
            ("exported", Linkage::Exported),
            ("internal", Linkage::Internal),
        ] {
            let function = program.create_function(name, linkage);
            let entry = program.function_mut(function).create_block();
            program
                .function_mut(function)
                .block_mut(entry)
                .set_terminator(Terminator::Return {
                    kind: ReturnKind::None,
                    frame_slots: 0,
                });
        }

        let binary = assemble_program(&program)
            .unwrap()
            .into_unit()
            .into_binary("test".to_owned())
            .unwrap();
        let serialized = binary.serialize().unwrap();
        let binary = FscriptBinary::deserialize(&serialized).unwrap();
        let (_, unit) = crate::AssemblyUnit::from_binary(binary).unwrap();
        // serializing writes function names in symbol table in B40
        assert_eq!(unit.function_offset("EXPORTED"), Some(0));
        assert_eq!(unit.function_offset("INTERNAL"), None);
        assert_eq!(unit.function_offset("internal"), None);
    }
}
