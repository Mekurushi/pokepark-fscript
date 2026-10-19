use super::block::BlockLowerer;
use crate::{
    CodegenError, CodegenResult,
    place::resolver::{IndexedOffset, ResolvedPlace, StorageBase},
};
use fsb_ir::{AddressingMode, Instruction, IntegerOperation, MemoryWidth, Word};
use fsc_mir::{ConstValue, Constant, Operand, Place, Ty};

impl BlockLowerer<'_, '_, '_> {
    pub(super) fn lower_operand(&mut self, operand: &Operand) -> CodegenResult<()> {
        match operand {
            Operand::Place(place) => self.lower_place(place)?,
            Operand::Constant(constant) => self.lower_constant(constant),
        }
        Ok(())
    }

    fn lower_place(&mut self, place: &Place) -> CodegenResult<()> {
        let place = self.places.resolve(place)?;
        match &place.ty {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str => self.lower_load_word(&place),
            Ty::Vec3 | Ty::Array { .. } => todo!("multi-slot operand lowering"),
            Ty::Void => todo!("void place reached code generation"),
        }
    }

    pub(super) fn lower_load_word(&mut self, place: &ResolvedPlace) -> CodegenResult<()> {
        match &place.base {
            StorageBase::Frame(base) => self.lower_load_frame_word(*base, place),
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

    pub(super) fn lower_store_word(&mut self, place: &ResolvedPlace) -> CodegenResult<()> {
        match &place.base {
            StorageBase::Frame(base) => self.lower_store_frame_word(*base, place),
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

    pub(super) fn lower_constant(&mut self, constant: &Constant) {
        let instruction = match &constant.value {
            ConstValue::Int(value) => Instruction::PushConstant(Word::from_i32(*value)),
            ConstValue::Float(value) => Instruction::PushConstant(Word::from_f32(*value)),
            ConstValue::Bool(value) => Instruction::PushConstant(Word::from_i32(i32::from(*value))),
            ConstValue::String(value) => Instruction::LoadString(value.clone()),
            ConstValue::Array(_) | ConstValue::Aggregate(_) => {
                todo!("aggregate constant lowering")
            }
        };
        self.output.push_instruction(instruction);
    }
}
