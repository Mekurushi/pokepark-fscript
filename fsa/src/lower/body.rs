use std::collections::HashMap;

use fsb_ir::{BlockId, FunctionRef, Program, Word};

use super::declarations::{DeclaredProgram, Symbols};
use super::error::{LowerError, LowerResult};
use crate::ast::{self, Item, Script};

pub(super) fn lower(script: &Script, declared: DeclaredProgram) -> LowerResult<Program> {
    let DeclaredProgram {
        mut program,
        symbols,
    } = declared;

    for item in &script.items {
        let Item::Function(source) = item else {
            continue;
        };
        let Some(FunctionRef::Internal(id)) = symbols.functions.get(&source.name) else {
            return Err(LowerError::UnknownFunction(source.name.clone()));
        };
        lower_function(program.function_mut(*id), &symbols, source)?;
    }

    Ok(program)
}

fn lower_function(
    target: &mut fsb_ir::Function,
    symbols: &Symbols,
    source: &ast::Function,
) -> LowerResult<()> {
    let entry = target.create_block();
    let labels = declare_labels(target, source)?;
    FunctionLowerer::new(target, symbols, source, labels, entry).lower()
}

fn declare_labels(
    target: &mut fsb_ir::Function,
    source: &ast::Function,
) -> LowerResult<HashMap<String, BlockId>> {
    let mut labels = HashMap::new();

    for statement in &source.body {
        let ast::Statement::Label(name) = statement else {
            continue;
        };
        if labels.contains_key(name) {
            return Err(LowerError::DuplicateLabel {
                function: source.name.clone(),
                label: name.clone(),
            });
        }
        labels.insert(name.clone(), target.create_block());
    }

    Ok(labels)
}

struct FunctionLowerer<'ir, 'symbols, 'ast> {
    target: &'ir mut fsb_ir::Function,
    symbols: &'symbols Symbols,
    source: &'ast ast::Function,
    labels: HashMap<String, BlockId>,
    current_block: Option<BlockId>,
}

impl<'ir, 'symbols, 'ast> FunctionLowerer<'ir, 'symbols, 'ast> {
    fn new(
        target: &'ir mut fsb_ir::Function,
        symbols: &'symbols Symbols,
        source: &'ast ast::Function,
        labels: HashMap<String, BlockId>,
        entry: BlockId,
    ) -> Self {
        Self {
            target,
            symbols,
            source,
            labels,
            current_block: Some(entry),
        }
    }

    fn lower(mut self) -> LowerResult<()> {
        for statement in &self.source.body {
            match statement {
                ast::Statement::Label(name) => self.define_label(name)?,
                ast::Statement::Instruction(instruction) => {
                    let instruction = lower_instruction(instruction, self.symbols)?;
                    let block = self.current_or_new_block();
                    self.target.block_mut(block).push_instruction(instruction);
                }
                ast::Statement::Terminator(terminator) => self.lower_terminator(terminator)?,
            }
        }

        if self.current_block.is_some() {
            return Err(LowerError::MissingTerminator {
                function: self.source.name.clone(),
            });
        }

        Ok(())
    }

    fn define_label(&mut self, name: &str) -> LowerResult<()> {
        let block = self.resolve_label(name)?;
        if let Some(current) = self.current_block.replace(block)
            && current != block
        {
            self.target
                .block_mut(current)
                .set_terminator(fsb_ir::Terminator::Jump(block));
        }
        Ok(())
    }

    fn resolve_label(&self, name: &str) -> LowerResult<BlockId> {
        self.labels
            .get(name)
            .copied()
            .ok_or_else(|| LowerError::UnknownLabel {
                function: self.source.name.clone(),
                label: name.to_owned(),
            })
    }

    fn lower_terminator(&mut self, terminator: &ast::Terminator) -> LowerResult<()> {
        let current = self.current_or_new_block();
        let (terminator, next) = match terminator {
            ast::Terminator::Jump(target) => (
                fsb_ir::Terminator::Jump(self.resolve_label(target)?),
                None,
            ),
            ast::Terminator::Branch { kind, target } => {
                let taken = self.resolve_label(target)?;
                let not_taken = self.target.create_block();
                (
                    fsb_ir::Terminator::Branch {
                        kind: lower_branch_kind(*kind),
                        taken,
                        not_taken,
                    },
                    Some(not_taken),
                )
            }
            ast::Terminator::Return { kind, frame_slots } => (
                fsb_ir::Terminator::Return {
                    kind: lower_return_kind(*kind),
                    frame_slots: *frame_slots,
                },
                None,
            ),
            ast::Terminator::Exit1 => (
                fsb_ir::Terminator::Exit(fsb_ir::ExitKind::Abort),
                None,
            ),
            ast::Terminator::Exit2 => (
                fsb_ir::Terminator::Exit(fsb_ir::ExitKind::Reset),
                None,
            ),
        };

        self.target.block_mut(current).set_terminator(terminator);
        self.current_block = next;
        Ok(())
    }

    fn current_or_new_block(&mut self) -> BlockId {
        if let Some(block) = self.current_block {
            block
        } else {
            let block = self.target.create_block();
            self.current_block = Some(block);
            block
        }
    }
}

fn lower_instruction(
    instruction: &ast::Instruction,
    symbols: &Symbols,
) -> LowerResult<fsb_ir::Instruction> {
    use ast::Instruction as Ast;
    use fsb_ir::Instruction as Ir;

    Ok(match instruction {
        Ast::Syscall {
            argument_count,
            page,
            function,
        } => Ir::Syscall {
            argument_count: *argument_count,
            page: *page,
            function: *function,
        },
        Ast::Delay(value) => Ir::Delay(*value),
        Ast::DelayFromStack => Ir::DelayFromStack,
        Ast::DelayIfNonZero => Ir::DelayIfNonZero,
        Ast::Call(name) => Ir::Call(
            symbols
                .functions
                .get(name)
                .cloned()
                .ok_or_else(|| LowerError::UnknownFunction(name.clone()))?,
        ),
        Ast::GrowStack(value) => Ir::GrowStack(*value),
        Ast::ShrinkStack(value) => Ir::ShrinkStack(*value),
        Ast::LoadFrameSlot(offset) => Ir::LoadFrameSlot(*offset),
        Ast::LoadFrameSlotAddress(offset) => Ir::LoadFrameSlotAddress(*offset),
        Ast::StoreFrameSlot(offset) => Ir::StoreFrameSlot(*offset),
        Ast::AddToFrameSlot(offset) => Ir::AddToFrameSlot(*offset),
        Ast::SubtractFromFrameSlot(offset) => Ir::SubtractFromFrameSlot(*offset),
        Ast::LoadRel(offset) => Ir::LoadRel(*offset),
        Ast::PushConstant(value) => Ir::PushConstant(Word::from_bits(*value)),
        Ast::PushResult => Ir::PushResult,
        Ast::LoadString(value) => Ir::LoadString(value.clone()),
        Ast::Integer(operation) => Ir::Integer(lower_integer_operation(*operation)),
        Ast::Float(operation) => Ir::Float(lower_float_operation(*operation)),
        Ast::IntegerCompare(comparison) => Ir::IntegerCompare(lower_comparison(*comparison)),
        Ast::FloatCompare(comparison) => Ir::FloatCompare(lower_comparison(*comparison)),
        Ast::Shift(operation) => Ir::Shift(lower_shift_operation(*operation)),
        Ast::LoadAddress(name) => Ir::LoadAddress(
            symbols
                .data
                .get(name)
                .cloned()
                .ok_or_else(|| LowerError::UnknownData(name.clone()))?,
        ),
        Ast::Load { width, addressing } => Ir::Load {
            width: lower_memory_width(*width),
            addressing: lower_addressing_mode(*addressing),
        },
        Ast::Store {
            width,
            addressing,
            operation,
        } => Ir::Store {
            width: lower_memory_width(*width),
            addressing: lower_addressing_mode(*addressing),
            operation: lower_store_operation(*operation),
        },
        Ast::Convert { kind, stack_offset } => Ir::Convert {
            kind: lower_conversion_kind(*kind),
            stack_offset: *stack_offset,
        },
        Ast::Debug { subtype, operand } => Ir::Debug {
            subtype: *subtype,
            operand: *operand,
        },
    })
}

fn lower_integer_operation(operation: ast::IntegerOperation) -> fsb_ir::IntegerOperation {
    match operation {
        ast::IntegerOperation::Add => fsb_ir::IntegerOperation::Add,
        ast::IntegerOperation::Subtract => fsb_ir::IntegerOperation::Subtract,
        ast::IntegerOperation::Multiply => fsb_ir::IntegerOperation::Multiply,
        ast::IntegerOperation::Divide => fsb_ir::IntegerOperation::Divide,
        ast::IntegerOperation::Remainder => fsb_ir::IntegerOperation::Remainder,
        ast::IntegerOperation::And => fsb_ir::IntegerOperation::And,
        ast::IntegerOperation::Or => fsb_ir::IntegerOperation::Or,
        ast::IntegerOperation::Xor => fsb_ir::IntegerOperation::Xor,
        ast::IntegerOperation::Not => fsb_ir::IntegerOperation::Not,
        ast::IntegerOperation::IsZero => fsb_ir::IntegerOperation::IsZero,
        ast::IntegerOperation::Negate => fsb_ir::IntegerOperation::Negate,
    }
}

fn lower_float_operation(operation: ast::FloatOperation) -> fsb_ir::FloatOperation {
    match operation {
        ast::FloatOperation::Add => fsb_ir::FloatOperation::Add,
        ast::FloatOperation::Subtract => fsb_ir::FloatOperation::Subtract,
        ast::FloatOperation::Multiply => fsb_ir::FloatOperation::Multiply,
        ast::FloatOperation::Divide => fsb_ir::FloatOperation::Divide,
        ast::FloatOperation::IsZero => fsb_ir::FloatOperation::IsZero,
        ast::FloatOperation::Negate => fsb_ir::FloatOperation::Negate,
    }
}

fn lower_comparison(comparison: ast::Comparison) -> fsb_ir::Comparison {
    match comparison {
        ast::Comparison::Equal => fsb_ir::Comparison::Equal,
        ast::Comparison::NotEqual => fsb_ir::Comparison::NotEqual,
        ast::Comparison::LessThan => fsb_ir::Comparison::LessThan,
        ast::Comparison::GreaterThan => fsb_ir::Comparison::GreaterThan,
        ast::Comparison::LessOrEqual => fsb_ir::Comparison::LessOrEqual,
        ast::Comparison::GreaterOrEqual => fsb_ir::Comparison::GreaterOrEqual,
    }
}

fn lower_shift_operation(operation: ast::ShiftOperation) -> fsb_ir::ShiftOperation {
    match operation {
        ast::ShiftOperation::Left => fsb_ir::ShiftOperation::Left,
        ast::ShiftOperation::RightArithmetic => fsb_ir::ShiftOperation::RightArithmetic,
        ast::ShiftOperation::RightLogical => fsb_ir::ShiftOperation::RightLogical,
    }
}

fn lower_memory_width(width: ast::MemoryWidth) -> fsb_ir::MemoryWidth {
    match width {
        ast::MemoryWidth::Byte => fsb_ir::MemoryWidth::Byte,
        ast::MemoryWidth::Halfword => fsb_ir::MemoryWidth::Halfword,
        ast::MemoryWidth::Word => fsb_ir::MemoryWidth::Word,
    }
}

fn lower_addressing_mode(mode: ast::AddressingMode) -> fsb_ir::AddressingMode {
    match mode {
        ast::AddressingMode::Direct => fsb_ir::AddressingMode::Direct,
        ast::AddressingMode::Indexed => fsb_ir::AddressingMode::Indexed,
    }
}

fn lower_store_operation(operation: ast::StoreOperation) -> fsb_ir::StoreOperation {
    match operation {
        ast::StoreOperation::Assign => fsb_ir::StoreOperation::Assign,
        ast::StoreOperation::Add => fsb_ir::StoreOperation::Add,
        ast::StoreOperation::Subtract => fsb_ir::StoreOperation::Subtract,
    }
}

fn lower_conversion_kind(kind: ast::ConversionKind) -> fsb_ir::ConversionKind {
    match kind {
        ast::ConversionKind::IntegerToFloat => fsb_ir::ConversionKind::IntegerToFloat,
        ast::ConversionKind::FloatToInteger => fsb_ir::ConversionKind::FloatToInteger,
    }
}

fn lower_branch_kind(kind: ast::BranchKind) -> fsb_ir::BranchKind {
    match kind {
        ast::BranchKind::IfZero(mode) => fsb_ir::BranchKind::IfZero(lower_branch_mode(mode)),
        ast::BranchKind::IfNonZero(mode) => {
            fsb_ir::BranchKind::IfNonZero(lower_branch_mode(mode))
        }
        ast::BranchKind::IfEqual => fsb_ir::BranchKind::IfEqual,
        ast::BranchKind::IfEqualImmediate(value) => fsb_ir::BranchKind::IfEqualImmediate(value),
    }
}

fn lower_branch_mode(mode: ast::BranchMode) -> fsb_ir::BranchMode {
    match mode {
        ast::BranchMode::Normal => fsb_ir::BranchMode::Normal,
        ast::BranchMode::Pause => fsb_ir::BranchMode::Pause,
        ast::BranchMode::Set => fsb_ir::BranchMode::Set,
    }
}

fn lower_return_kind(kind: ast::ReturnKind) -> fsb_ir::ReturnKind {
    match kind {
        ast::ReturnKind::None => fsb_ir::ReturnKind::None,
        ast::ReturnKind::Value => fsb_ir::ReturnKind::Value,
    }
}
