use super::{body::BodyBuilder, context::lower_source};
use crate::{
    BinaryOp, ConstValue, Constant, FieldOperand, Operand, Place, ProjectionElem, RETURN_PLACE,
    Rvalue, Statement, StatementKind, UnaryOp,
};
use fsc_sema::{thir, types::Ty};

impl BodyBuilder<'_> {
    pub(super) fn lower_operand(&mut self, expression: thir::ExprId) -> Operand {
        match &self.thir.expressions[expression].kind {
            thir::ExprKind::Constant(value) => {
                let value = lower_const(value);
                let ty = self.thir.expressions[expression].ty.clone();
                Operand::Constant(Constant { value, ty })
            }
            thir::ExprKind::Local(_) => Operand::Place(self.lower_place(expression)),
            thir::ExprKind::Field { base, field } => {
                let (base, field) = (*base, *field);
                Operand::Place(self.lower_field_value(base, field))
            }
            thir::ExprKind::Array { .. }
            | thir::ExprKind::Aggregate { .. }
            | thir::ExprKind::Unary { .. }
            | thir::ExprKind::Binary {
                op:
                    thir::BinaryOp::Add
                    | thir::BinaryOp::Subtract
                    | thir::BinaryOp::Multiply
                    | thir::BinaryOp::Divide
                    | thir::BinaryOp::Equal
                    | thir::BinaryOp::NotEqual
                    | thir::BinaryOp::Less
                    | thir::BinaryOp::Greater
                    | thir::BinaryOp::LessEqual
                    | thir::BinaryOp::GreaterEqual,
                ..
            } => {
                let (ty, source) = {
                    let expression = &self.thir.expressions[expression];
                    (expression.ty.clone(), expression.source)
                };
                let value = self.lower_rvalue(expression);
                Operand::Place(self.materialize(value, &ty, source))
            }
            thir::ExprKind::Binary {
                op: thir::BinaryOp::LogicalAnd | thir::BinaryOp::LogicalOr,
                ..
            } => self.lower_condition_to_operand(expression),
            thir::ExprKind::Call { .. } | thir::ExprKind::RawSyscall { .. } => {
                let (ty, source) = {
                    let expression = &self.thir.expressions[expression];
                    (expression.ty.clone(), expression.source)
                };
                let destination = Place {
                    local: self.allocate_temporary(&ty, source),
                    projection: Box::new([]),
                };
                self.lower_effect(expression, Some(destination.clone()));
                Operand::Place(destination)
            }
        }
    }

    fn lower_rvalue(&mut self, expression: thir::ExprId) -> Rvalue {
        match &self.thir.expressions[expression].kind {
            thir::ExprKind::Constant(_)
            | thir::ExprKind::Local(_)
            | thir::ExprKind::Field { .. } => Rvalue::Use(self.lower_operand(expression)),
            thir::ExprKind::Array { elements } => {
                let elements = elements.clone();
                Rvalue::Array {
                    elements: elements
                        .iter()
                        .map(|element| self.lower_operand(*element))
                        .collect(),
                }
            }
            thir::ExprKind::Aggregate { fields } => {
                let fields = fields.clone();
                let ty = self.thir.expressions[expression].ty.clone();
                Rvalue::Aggregate {
                    ty,
                    fields: fields
                        .iter()
                        .map(|field| FieldOperand {
                            field: field.field,
                            value: self.lower_operand(field.value),
                        })
                        .collect(),
                }
            }
            thir::ExprKind::Unary { op, operand } => {
                let (op, operand) = (*op, *operand);
                Rvalue::Unary {
                    op: lower_unary(op),
                    operand: self.lower_operand(operand),
                }
            }
            thir::ExprKind::Binary {
                op: thir::BinaryOp::LogicalAnd | thir::BinaryOp::LogicalOr,
                ..
            } => unreachable!("logical expressions require CFG lowering"),
            thir::ExprKind::Binary { op, lhs, rhs } => {
                let (op, lhs, rhs) = (*op, *lhs, *rhs);
                Rvalue::Binary {
                    op: lower_binary(op),
                    lhs: self.lower_operand(lhs),
                    rhs: self.lower_operand(rhs),
                }
            }
            thir::ExprKind::Call { .. } | thir::ExprKind::RawSyscall { .. } => {
                unreachable!("effectful expressions are not rvalues")
            }
        }
    }

    pub(super) fn lower_expression_into(&mut self, expression: thir::ExprId, destination: Place) {
        if matches!(
            &self.thir.expressions[expression].kind,
            thir::ExprKind::Call { .. } | thir::ExprKind::RawSyscall { .. }
        ) {
            self.lower_effect(expression, Some(destination));
        } else if matches!(
            &self.thir.expressions[expression].kind,
            thir::ExprKind::Binary {
                op: thir::BinaryOp::LogicalAnd | thir::BinaryOp::LogicalOr,
                ..
            }
        ) {
            self.lower_condition_into(expression, destination);
        } else {
            let value = self.lower_rvalue(expression);
            let source = self.thir.expressions[expression].source;
            self.cfg.push_statement(Statement {
                kind: StatementKind::Assign {
                    target: destination,
                    value,
                },
                source: lower_source(source),
            });
        }
    }

    pub(super) fn lower_expression_statement(&mut self, expression: thir::ExprId) {
        if matches!(
            &self.thir.expressions[expression].kind,
            thir::ExprKind::Call { .. } | thir::ExprKind::RawSyscall { .. }
        ) {
            self.lower_effect(expression, None);
        } else {
            let _ = self.lower_operand(expression);
        }
    }

    fn lower_effect(&mut self, expression: thir::ExprId, destination: Option<Place>) {
        let source = self.thir.expressions[expression].source;
        let kind = match &self.thir.expressions[expression].kind {
            thir::ExprKind::Call { callee, arguments } => {
                let (callee, arguments) = (*callee, arguments.clone());
                StatementKind::Call {
                    destination,
                    callee: self.mapped_function(callee),
                    arguments: self.lower_arguments(&arguments),
                }
            }
            thir::ExprKind::RawSyscall {
                page,
                function,
                arguments,
            } => {
                let (page, function, arguments) = (*page, *function, arguments.clone());
                StatementKind::RawSyscall {
                    destination,
                    page: page.get(),
                    function: function.get(),
                    arguments: self.lower_arguments(&arguments),
                }
            }
            _ => unreachable!("only effectful expressions lower as effect statements"),
        };

        self.cfg.push_statement(Statement {
            kind,
            source: lower_source(source),
        });
    }

    fn lower_arguments(&mut self, expressions: &[thir::ExprId]) -> Box<[Operand]> {
        expressions
            .iter()
            .map(|expression| self.lower_operand(*expression))
            .collect()
    }

    pub(super) fn lower_place(&self, expression: thir::ExprId) -> Place {
        match &self.thir.expressions[expression].kind {
            thir::ExprKind::Local(local) => Place {
                local: self.mapped_local(*local),
                projection: Box::new([]),
            },
            thir::ExprKind::Field { base, field } => append_field(self.lower_place(*base), *field),
            _ => unreachable!("THIR place must be rooted in a local"),
        }
    }

    pub(super) fn local_place(&self, local: thir::LocalId) -> Place {
        Place {
            local: self.mapped_local(local),
            projection: Box::new([]),
        }
    }

    pub(super) fn return_place() -> Place {
        Place {
            local: RETURN_PLACE,
            projection: Box::new([]),
        }
    }

    fn lower_field_value(&mut self, base: thir::ExprId, field: fsc_sema::types::FieldIdx) -> Place {
        let (base_ty, base_source) = {
            let base_expression = &self.thir.expressions[base];
            (base_expression.ty.clone(), base_expression.source)
        };
        let place = match self.lower_operand(base) {
            Operand::Place(place) => place,
            Operand::Constant(constant) => self.materialize(
                Rvalue::Use(Operand::Constant(constant)),
                &base_ty,
                base_source,
            ),
        };

        append_field(place, field)
    }

    fn materialize(&mut self, value: Rvalue, ty: &Ty, source: thir::SourceInfo) -> Place {
        let destination = Place {
            local: self.allocate_temporary(ty, source),
            projection: Box::new([]),
        };

        self.cfg.push_statement(Statement {
            kind: StatementKind::Assign {
                target: destination.clone(),
                value,
            },
            source: lower_source(source),
        });

        destination
    }
}

fn append_field(mut place: Place, field: fsc_sema::types::FieldIdx) -> Place {
    let mut projection = place.projection.into_vec();
    projection.push(ProjectionElem::Field(field));
    place.projection = projection.into_boxed_slice();
    place
}

fn lower_const(value: &thir::ConstValue) -> ConstValue {
    match value {
        thir::ConstValue::Int(value) => ConstValue::Int(*value),
        thir::ConstValue::Float(value) => ConstValue::Float(*value),
        thir::ConstValue::Bool(value) => ConstValue::Bool(*value),
        thir::ConstValue::String(value) => ConstValue::String(value.clone()),
        thir::ConstValue::Array(elements) => {
            ConstValue::Array(elements.iter().map(lower_const).collect())
        }
        thir::ConstValue::Aggregate(fields) => {
            ConstValue::Aggregate(fields.iter().map(lower_const).collect())
        }
    }
}

const fn lower_unary(op: thir::UnaryOp) -> UnaryOp {
    match op {
        thir::UnaryOp::Not => UnaryOp::Not,
        thir::UnaryOp::Negate => UnaryOp::Negate,
    }
}

fn lower_binary(op: thir::BinaryOp) -> BinaryOp {
    match op {
        thir::BinaryOp::Add => BinaryOp::Add,
        thir::BinaryOp::Subtract => BinaryOp::Subtract,
        thir::BinaryOp::Multiply => BinaryOp::Multiply,
        thir::BinaryOp::Divide => BinaryOp::Divide,
        thir::BinaryOp::Equal => BinaryOp::Equal,
        thir::BinaryOp::NotEqual => BinaryOp::NotEqual,
        thir::BinaryOp::Less => BinaryOp::Less,
        thir::BinaryOp::Greater => BinaryOp::Greater,
        thir::BinaryOp::LessEqual => BinaryOp::LessEqual,
        thir::BinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
        thir::BinaryOp::LogicalAnd | thir::BinaryOp::LogicalOr => {
            unreachable!("logical expressions require CFG lowering")
        }
    }
}
