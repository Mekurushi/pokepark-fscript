use super::block::BlockLowerer;
use crate::{
    CodegenError, CodegenResult,
    place::resolver::{IndexedOffset, ResolvedPlace, StorageBase},
};
use fsb_ir::{AddressingMode, Instruction, IntegerOperation, MemoryWidth, Word};
use fsc_mir::{ConstValue, Operand, Ty};

const WORD_BYTES: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScalarKind {
    Integer,
    Float,
    Bool,
    String,
}

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_scalar_operand(&mut self, operand: &Operand) -> CodegenResult<ScalarKind> {
        match operand {
            Operand::Place(place) => {
                let place = self.function.resolve_place(place)?;
                let kind = scalar_kind(&place.ty);
                self.lower_load_word(&place)?;
                Ok(kind)
            }
            Operand::Constant(constant) => {
                let kind = scalar_kind(&constant.ty);
                self.lower_scalar_constant_value(&constant.value);
                Ok(kind)
            }
        }
    }

    pub(super) fn lower_load_word(&mut self, place: &ResolvedPlace) -> CodegenResult<()> {
        match &place.base {
            StorageBase::Frame(base) => self.lower_load_frame_word(*base, place),
            StorageBase::FramePointer(base) => {
                self.lower_load_indirect_frame_word(*base, place);
                Ok(())
            }
            StorageBase::Data(base) => {
                self.lower_load_data_word(base, place);
                Ok(())
            }
        }
    }

    fn lower_load_frame_word(
        &mut self,
        base: crate::frame::FrameSlot,
        place: &ResolvedPlace,
    ) -> CodegenResult<()> {
        if place.indexed_offsets.is_empty() {
            let offset =
                i16::try_from(place.static_offset).map_err(|_| CodegenError::FrameTooLarge)?;
            self.output.push_instruction(Instruction::LoadFrameSlot(
                base.checked_add(offset)?.offset(),
            ));
            return Ok(());
        }

        self.output
            .push_instruction(Instruction::LoadFrameSlotAddress(base.offset()));
        let addressing = self.lower_address_offset(place.static_offset, &place.indexed_offsets);
        self.output.push_instruction(Instruction::Load {
            width: MemoryWidth::Word,
            addressing,
        });
        Ok(())
    }

    fn lower_load_indirect_frame_word(
        &mut self,
        base: crate::frame::FrameSlot,
        place: &ResolvedPlace,
    ) {
        self.output
            .push_instruction(Instruction::LoadFrameSlot(base.offset()));
        let addressing = self.lower_address_offset(place.static_offset, &place.indexed_offsets);
        self.output.push_instruction(Instruction::Load {
            width: MemoryWidth::Word,
            addressing,
        });
    }

    fn lower_load_data_word(&mut self, base: &fsb_ir::DataRef, place: &ResolvedPlace) {
        self.output
            .push_instruction(Instruction::LoadAddress(base.clone()));
        let addressing = self.lower_address_offset(place.static_offset, &place.indexed_offsets);
        self.output.push_instruction(Instruction::Load {
            width: MemoryWidth::Word,
            addressing,
        });
    }

    fn lower_address_offset(
        &mut self,
        static_offset: u16,
        indexed_offsets: &[IndexedOffset],
    ) -> AddressingMode {
        let mut has_offset = false;
        if static_offset != 0 {
            self.output
                .push_instruction(Instruction::PushConstant(Word::from(static_offset)));
            has_offset = true;
        }

        for offset in indexed_offsets {
            self.output
                .push_instruction(Instruction::LoadFrameSlot(offset.index.offset()));

            if offset.stride != 1 {
                self.output
                    .push_instruction(Instruction::PushConstant(Word::from(offset.stride)));
                self.output
                    .push_instruction(Instruction::Integer(IntegerOperation::Multiply));
            }
            if has_offset {
                self.output
                    .push_instruction(Instruction::Integer(IntegerOperation::Add));
            }
            has_offset = true;
        }

        if has_offset {
            AddressingMode::Indexed
        } else {
            AddressingMode::Direct
        }
    }

    pub(super) fn lower_place_address(&mut self, place: &ResolvedPlace) {
        match &place.base {
            StorageBase::Frame(base) => self
                .output
                .push_instruction(Instruction::LoadFrameSlotAddress(base.offset())),
            StorageBase::FramePointer(base) => self
                .output
                .push_instruction(Instruction::LoadFrameSlot(base.offset())),
            StorageBase::Data(base) => self
                .output
                .push_instruction(Instruction::LoadAddress(base.clone())),
        }

        let mut has_offset = false;
        if place.static_offset != 0 {
            self.output
                .push_instruction(Instruction::PushConstant(Word::from(place.static_offset)));
            has_offset = true;
        }

        for offset in &place.indexed_offsets {
            self.output
                .push_instruction(Instruction::LoadFrameSlot(offset.index.offset()));

            if offset.stride != 1 {
                self.output
                    .push_instruction(Instruction::PushConstant(Word::from(offset.stride)));
                self.output
                    .push_instruction(Instruction::Integer(IntegerOperation::Multiply));
            }
            if has_offset {
                self.output
                    .push_instruction(Instruction::Integer(IntegerOperation::Add));
            }
            has_offset = true;
        }

        if has_offset {
            self.output
                .push_instruction(Instruction::PushConstant(Word::from(WORD_BYTES)));
            self.output
                .push_instruction(Instruction::Integer(IntegerOperation::Multiply));
            self.output
                .push_instruction(Instruction::Integer(IntegerOperation::Add));
        }
    }

    pub(super) fn lower_store_word(&mut self, place: &ResolvedPlace) -> CodegenResult<()> {
        match &place.base {
            StorageBase::Frame(base) => self.lower_store_frame_word(*base, place),
            StorageBase::FramePointer(base) => {
                self.lower_store_indirect_frame_word(*base, place);
                Ok(())
            }
            StorageBase::Data(base) => {
                self.lower_store_data_word(base, place);
                Ok(())
            }
        }
    }

    fn lower_store_frame_word(
        &mut self,
        base: crate::frame::FrameSlot,
        place: &ResolvedPlace,
    ) -> CodegenResult<()> {
        if place.indexed_offsets.is_empty() {
            let offset =
                i16::try_from(place.static_offset).map_err(|_| CodegenError::FrameTooLarge)?;
            self.output.push_instruction(Instruction::StoreFrameSlot(
                base.checked_add(offset)?.offset(),
            ));
            return Ok(());
        }

        self.output
            .push_instruction(Instruction::LoadFrameSlotAddress(base.offset()));
        let addressing = self.lower_address_offset(place.static_offset, &place.indexed_offsets);
        self.output.push_instruction(Instruction::Store {
            width: MemoryWidth::Word,
            addressing,
            operation: fsb_ir::StoreOperation::Assign,
        });
        Ok(())
    }

    fn lower_store_indirect_frame_word(
        &mut self,
        base: crate::frame::FrameSlot,
        place: &ResolvedPlace,
    ) {
        self.output
            .push_instruction(Instruction::LoadFrameSlot(base.offset()));
        let addressing = self.lower_address_offset(place.static_offset, &place.indexed_offsets);
        self.output.push_instruction(Instruction::Store {
            width: MemoryWidth::Word,
            addressing,
            operation: fsb_ir::StoreOperation::Assign,
        });
    }

    fn lower_store_data_word(&mut self, base: &fsb_ir::DataRef, place: &ResolvedPlace) {
        self.output
            .push_instruction(Instruction::LoadAddress(base.clone()));
        let addressing = self.lower_address_offset(place.static_offset, &place.indexed_offsets);
        self.output.push_instruction(Instruction::Store {
            width: MemoryWidth::Word,
            addressing,
            operation: fsb_ir::StoreOperation::Assign,
        });
    }

    pub(super) fn lower_scalar_constant_value(&mut self, value: &ConstValue) {
        let instruction = match value {
            ConstValue::Int(value) => Instruction::PushConstant(Word::from_i32(*value)),
            ConstValue::Float(value) => Instruction::PushConstant(Word::from_f32(*value)),
            ConstValue::Bool(value) => Instruction::PushConstant(Word::from_i32(i32::from(*value))),
            ConstValue::String(value) => Instruction::LoadString(value.clone()),
            ConstValue::Array(_) | ConstValue::Aggregate(_) => {
                unreachable!("aggregate/array constant reached scalar constant lowering")
            }
        };
        self.output.push_instruction(instruction);
    }
}

fn scalar_kind(ty: &Ty) -> ScalarKind {
    match ty {
        Ty::Int => ScalarKind::Integer,
        Ty::Float => ScalarKind::Float,
        Ty::Bool => ScalarKind::Bool,
        Ty::Str => ScalarKind::String,
        Ty::Vec3 | Ty::Array { .. } | Ty::Void => {
            unreachable!("MIR scalar operand must have scalar type")
        }
    }
}
