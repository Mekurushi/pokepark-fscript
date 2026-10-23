use super::{block::BlockLowerer, operand::ScalarKind};
use crate::{CodegenError, CodegenResult, place::resolver::ResolvedPlace};
use fsb_ir::{Comparison, FloatOperation, Instruction, IntegerOperation};
use fsc_mir::{
    BinaryOp, ConstValue, Constant, FieldOperand, Operand, Rvalue, Ty, UnaryOp, aggregate_fields,
};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_rvalue_into(
        &mut self,
        value: &Rvalue,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match value {
            Rvalue::Use(operand @ Operand::Place(source)) => {
                let function = self.function;
                if let Some(value) = function.stackified_rvalue(source) {
                    self.lower_scalar_rvalue_to_stack(value)?;
                    return self.lower_store_word(target);
                }
                self.lower_operand_into(operand, target)
            }
            Rvalue::Use(operand) => self.lower_operand_into(operand, target),
            Rvalue::Unary { .. } | Rvalue::Binary { .. } => {
                self.lower_scalar_rvalue_to_stack(value)?;
                self.lower_store_word(target)
            }
            Rvalue::Array { elements } => self.lower_array_into(elements, target),
            Rvalue::Aggregate { ty, fields } => self.lower_aggregate_into(ty, fields, target),
        }
    }

    pub(super) fn lower_scalar_rvalue_to_stack(
        &mut self,
        value: &Rvalue,
    ) -> CodegenResult<ScalarKind> {
        match value {
            Rvalue::Use(operand) => self.lower_scalar_operand(operand),
            Rvalue::Unary { op, operand } => {
                let operand = self.lower_scalar_operand(operand)?;
                self.output
                    .push_instruction(lower_unary_instruction(*op, operand));
                Ok(operand)
            }
            Rvalue::Binary { op, lhs, rhs } => {
                let lhs = self.lower_scalar_operand(lhs)?;
                let rhs = self.lower_scalar_operand(rhs)?;
                debug_assert_eq!(lhs, rhs, "MIR binary operands must have the same type");

                self.output
                    .push_instruction(lower_binary_instruction(*op, lhs));

                let result = match op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        lhs
                    }
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::Greater
                    | BinaryOp::LessEqual
                    | BinaryOp::GreaterEqual => ScalarKind::Bool,
                };
                Ok(result)
            }
            Rvalue::Array { .. } | Rvalue::Aggregate { .. } => {
                unreachable!("aggregate MIR rvalue reached scalar lowering")
            }
        }
    }

    fn lower_array_into(
        &mut self,
        elements: &[Operand],
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        let Ty::Array { length, .. } = &target.ty else {
            return Err(CodegenError::UnsupportedType(target.ty.clone()));
        };
        debug_assert_eq!(
            elements.len(),
            *length,
            "MIR array rvalue must initialize every element"
        );

        for (index, element) in elements.iter().enumerate() {
            let target = target.project_constant_index(index)?;
            self.lower_operand_into(element, &target)?;
        }
        Ok(())
    }

    fn lower_aggregate_into(
        &mut self,
        ty: &Ty,
        fields: &[FieldOperand],
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match ty {
            Ty::Vec3 => self.lower_vec3_into(fields, target),
            _ => self.lower_aggregate_fields_into(fields, target),
        }
    }

    fn lower_aggregate_fields_into(
        &mut self,
        fields: &[FieldOperand],
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        for field in fields {
            let target = target.project_field(field.field)?;
            self.lower_operand_into(&field.value, &target)?;
        }
        Ok(())
    }

    fn lower_vec3_into(
        &mut self,
        fields: &[FieldOperand],
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        const PAGE: u8 = 3;
        const FUNCTION: u16 = 2;

        let definitions = aggregate_fields(&Ty::Vec3)
            .unwrap_or_else(|| unreachable!("vec3 must have an aggregate definition"));

        for definition in definitions.iter().rev() {
            let field = fields
                .iter()
                .find(|field| field.field == definition.index())
                .unwrap_or_else(|| unreachable!("MIR vec3 aggregate must initialize every field"));
            let kind = self.lower_scalar_operand(&field.value)?;
            debug_assert_eq!(kind, ScalarKind::Float, "vec3 fields must be floats");
        }

        self.lower_place_address(target);
        self.output.push_instruction(Instruction::Syscall {
            argument_count: 4,
            page: PAGE,
            function: FUNCTION,
        });
        Ok(())
    }

    fn copy_vec3_into(
        &mut self,
        source: &ResolvedPlace,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        const PAGE: u8 = 3;
        const FUNCTION: u16 = 2;

        let definitions = aggregate_fields(&Ty::Vec3)
            .unwrap_or_else(|| unreachable!("vec3 must have an aggregate definition"));

        for definition in definitions.iter().rev() {
            let source = source.project_field(definition.index())?;
            self.lower_load_word(&source)?;
        }

        self.lower_place_address(target);
        self.output.push_instruction(Instruction::Syscall {
            argument_count: 4,
            page: PAGE,
            function: FUNCTION,
        });
        Ok(())
    }

    fn lower_operand_into(
        &mut self,
        operand: &Operand,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match operand {
            Operand::Place(source) => {
                let source = self.function.resolve_place(source)?;
                self.copy_place(&source, target)
            }
            Operand::Constant(constant) => self.lower_constant_into(constant, target),
        }
    }

    fn copy_place(&mut self, source: &ResolvedPlace, target: &ResolvedPlace) -> CodegenResult<()> {
        match &source.ty {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str => {
                self.lower_load_word(source)?;
                self.lower_store_word(target)
            }
            Ty::Vec3 => self.copy_vec3_into(source, target),
            Ty::Array { length, .. } => {
                for index in 0..*length {
                    let source = source.project_constant_index(index)?;
                    let target = target.project_constant_index(index)?;
                    self.copy_place(&source, &target)?;
                }
                Ok(())
            }
            Ty::Reference(_) | Ty::Void => unreachable!("unsupported place reached codegen"),
        }
    }

    fn lower_constant_into(
        &mut self,
        constant: &Constant,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        self.lower_constant_value_into(&constant.value, &constant.ty, target)
    }

    fn lower_constant_value_into(
        &mut self,
        value: &ConstValue,
        ty: &Ty,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match (value, ty) {
            (ConstValue::Int(_), Ty::Int)
            | (ConstValue::Float(_), Ty::Float)
            | (ConstValue::Bool(_), Ty::Bool)
            | (ConstValue::String(_), Ty::Str) => {
                self.lower_scalar_constant_value(value);
                self.lower_store_word(target)
            }
            (ConstValue::Array(values), Ty::Array { element, length }) => {
                debug_assert_eq!(
                    values.len(),
                    *length,
                    "MIR array constant must contain every element"
                );

                for (index, value) in values.iter().enumerate() {
                    let target = target.project_constant_index(index)?;
                    self.lower_constant_value_into(value, element, &target)?;
                }
                Ok(())
            }
            (ConstValue::Aggregate(values), ty) => {
                let fields = aggregate_fields(ty)
                    .ok_or_else(|| CodegenError::UnsupportedType(ty.clone()))?;
                debug_assert_eq!(
                    values.len(),
                    fields.len(),
                    "MIR aggregate constant must contain every field"
                );

                // invariant: expects fields and values to be equally ordered
                // TODO: better way to structure layout
                for (field, value) in fields.iter().zip(values) {
                    let target = target.project_field(field.index())?;
                    self.lower_constant_value_into(value, field.ty(), &target)?;
                }
                Ok(())
            }
            _ => Err(CodegenError::UnsupportedType(ty.clone())),
        }
    }
}

fn lower_unary_instruction(op: UnaryOp, operand: ScalarKind) -> Instruction {
    match (op, operand) {
        (UnaryOp::Not, ScalarKind::Bool) => Instruction::Integer(IntegerOperation::IsZero),
        (UnaryOp::Negate, ScalarKind::Integer) => Instruction::Integer(IntegerOperation::Negate),
        (UnaryOp::Negate, ScalarKind::Float) => Instruction::Float(FloatOperation::Negate),
        _ => unreachable!("unsupported unary operand reached codegen"),
    }
}

fn lower_binary_instruction(op: BinaryOp, operands: ScalarKind) -> Instruction {
    match (op, operands) {
        (BinaryOp::Add, ScalarKind::Integer) => Instruction::Integer(IntegerOperation::Add),
        (BinaryOp::Subtract, ScalarKind::Integer) => {
            Instruction::Integer(IntegerOperation::Subtract)
        }
        (BinaryOp::Multiply, ScalarKind::Integer) => {
            Instruction::Integer(IntegerOperation::Multiply)
        }
        (BinaryOp::Divide, ScalarKind::Integer) => Instruction::Integer(IntegerOperation::Divide),
        (BinaryOp::Add, ScalarKind::Float) => Instruction::Float(FloatOperation::Add),
        (BinaryOp::Subtract, ScalarKind::Float) => Instruction::Float(FloatOperation::Subtract),
        (BinaryOp::Multiply, ScalarKind::Float) => Instruction::Float(FloatOperation::Multiply),
        (BinaryOp::Divide, ScalarKind::Float) => Instruction::Float(FloatOperation::Divide),
        (BinaryOp::Equal, ScalarKind::Integer | ScalarKind::Bool) => {
            Instruction::IntegerCompare(Comparison::Equal)
        }
        (BinaryOp::NotEqual, ScalarKind::Integer | ScalarKind::Bool) => {
            Instruction::IntegerCompare(Comparison::NotEqual)
        }
        (BinaryOp::Less, ScalarKind::Integer) => Instruction::IntegerCompare(Comparison::LessThan),
        (BinaryOp::Greater, ScalarKind::Integer) => {
            Instruction::IntegerCompare(Comparison::GreaterThan)
        }
        (BinaryOp::LessEqual, ScalarKind::Integer) => {
            Instruction::IntegerCompare(Comparison::LessOrEqual)
        }
        (BinaryOp::GreaterEqual, ScalarKind::Integer) => {
            Instruction::IntegerCompare(Comparison::GreaterOrEqual)
        }
        (BinaryOp::Equal, ScalarKind::Float) => Instruction::FloatCompare(Comparison::Equal),
        (BinaryOp::NotEqual, ScalarKind::Float) => Instruction::FloatCompare(Comparison::NotEqual),
        (BinaryOp::Less, ScalarKind::Float) => Instruction::FloatCompare(Comparison::LessThan),
        (BinaryOp::Greater, ScalarKind::Float) => {
            Instruction::FloatCompare(Comparison::GreaterThan)
        }
        (BinaryOp::LessEqual, ScalarKind::Float) => {
            Instruction::FloatCompare(Comparison::LessOrEqual)
        }
        (BinaryOp::GreaterEqual, ScalarKind::Float) => {
            Instruction::FloatCompare(Comparison::GreaterOrEqual)
        }
        _ => unreachable!("unsupported binary operands reached codegen"),
    }
}
